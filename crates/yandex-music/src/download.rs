//! Track download information and direct link construction.
//!
//! The legacy API returns a set of [`DownloadInfo`] entries. Each entry points
//! to a small XML document containing the storage host, the file path, a
//! timestamp and a signature. The direct MP3 URL is reconstructed exactly like
//! the official clients do:
//!
//! ```text
//! sign = md5(SALT + path.without_leading_slash() + s)
//! url  = https://{host}/get-mp3/{sign}/{ts}{path}
//! ```
//!
//! For lossless playback the client uses the signed `/get-file-info` endpoint
//! instead. It returns a direct URL and is signed with HMAC-SHA256 over
//! timestamp, track id, quality, codecs and transport.

use crate::{Error, Result};
use base64::Engine as _;
use hmac::{Hmac, Mac};
use md5::{Digest, Md5};
use serde::Deserialize;
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

/// Key used by the desktop client to sign `/get-file-info` requests.
pub const FILE_INFO_SIGN_KEY: &str = "kzqU4XhfCaY6B6JTHODeq5";
/// Client header tied to [`FILE_INFO_SIGN_KEY`].
pub const FILE_INFO_CLIENT: &str = "YandexMusicDesktopAppWindows/5.95.0";
/// Codecs requested for lossless playback. Raw FLAC is preferred; MP3 is the
/// fallback when the track or account has no lossless version.
pub const FILE_INFO_CODECS: &[&str] = &["flac", "flac-mp4", "mp3"];
/// Key used by the Android app to sign the lyrics endpoint.
pub const LYRICS_SIGN_KEY: &str = "p93jhgh689SBReK6ghtw62";

/// Salt used by Yandex to sign direct download URLs.
pub const SIGN_SALT: &str = "XGRlBW9FXlekgbPrRHuSiA";

/// One available encoding of a track.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadInfo {
    /// Codec, for example `mp3`.
    #[serde(default)]
    pub codec: String,
    /// Bitrate in kilobits per second.
    #[serde(default)]
    pub bitrate_in_kbps: u32,
    #[serde(default)]
    pub gain: bool,
    /// Whether this is only a short preview.
    #[serde(default)]
    pub preview: bool,
    /// URL of the XML document with the storage details.
    #[serde(default)]
    pub download_info_url: String,
    #[serde(default)]
    pub direct: bool,
}

impl DownloadInfo {
    /// Whether this variant is a full-length MP3 file.
    pub fn is_full_mp3(&self) -> bool {
        self.codec.eq_ignore_ascii_case("mp3") && !self.preview
    }
}

/// Response of the signed `/get-file-info` endpoint.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    #[serde(default)]
    pub download_info: Option<FileDownloadInfo>,
}

/// One direct file variant returned by `/get-file-info`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDownloadInfo {
    /// `lossless`, `hq` or `nq`.
    #[serde(default)]
    pub quality: String,
    /// `flac`, `flac-mp4`, `aac`, `mp3`, ...
    #[serde(default)]
    pub codec: String,
    /// Bitrate in kilobits per second (`0` for some MP4 containers).
    #[serde(default)]
    pub bitrate: u32,
    /// `raw` or `encraw`.
    #[serde(default)]
    pub transport: String,
    /// Direct URL of the file.
    #[serde(default)]
    pub url: String,
    /// Mirrors of [`Self::url`].
    #[serde(default)]
    pub urls: Vec<String>,
    #[serde(default)]
    pub gain: bool,
}

impl FileDownloadInfo {
    /// First usable direct URL.
    pub fn direct_url(&self) -> Option<&str> {
        if !self.url.is_empty() {
            Some(&self.url)
        } else {
            self.urls
                .iter()
                .find(|url| !url.is_empty())
                .map(String::as_str)
        }
    }

    /// Whether this variant is raw FLAC.
    pub fn is_lossless(&self) -> bool {
        self.codec.eq_ignore_ascii_case("flac") || self.codec.eq_ignore_ascii_case("flac-mp4")
    }

    /// Whether the variant can be decoded by rodio without extra decryption.
    pub fn is_playable(&self) -> bool {
        let raw = !self.transport.eq_ignore_ascii_case("encraw");
        let codec = self.codec.eq_ignore_ascii_case("flac")
            || self.codec.eq_ignore_ascii_case("flac-mp4")
            || self.codec.eq_ignore_ascii_case("mp3");
        raw && codec && self.direct_url().is_some()
    }
}

impl FileInfo {
    /// The direct variant suitable for playback.
    pub fn playable(&self) -> Option<&FileDownloadInfo> {
        self.download_info
            .as_ref()
            .filter(|info| info.is_playable())
    }
}

/// Sign `/get-file-info` at a fixed timestamp (used by tests and the client).
pub fn file_info_sign_at(
    track_id: &str,
    quality: &str,
    codecs: &[&str],
    transport: &str,
    timestamp: u64,
    key: &str,
) -> String {
    type HmacSha256 = Hmac<Sha256>;

    let message = format!(
        "{timestamp}{track_id}{quality}{}{transport}",
        codecs.concat()
    );
    let mut mac =
        HmacSha256::new_from_slice(key.as_bytes()).expect("HMAC accepts keys of any length");
    mac.update(message.as_bytes());
    let digest = mac.finalize().into_bytes();

    // The Python library strips the trailing `=` from padded base64. For a
    // 32-byte SHA-256 digest this is exactly unpadded base64.
    base64::engine::general_purpose::STANDARD_NO_PAD.encode(digest)
}

/// Sign `/get-file-info` for the current Unix timestamp.
/// Sign a lyrics request for a fixed timestamp. Unlike file-info signatures,
/// this one keeps the standard base64 padding.
pub fn lyrics_sign_at(track_id: &str, timestamp: u64, key: &str) -> String {
    type HmacSha256 = Hmac<Sha256>;

    let mut mac =
        HmacSha256::new_from_slice(key.as_bytes()).expect("HMAC accepts keys of any length");
    mac.update(format!("{track_id}{timestamp}").as_bytes());
    base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes())
}

/// Create a timestamp and signature for `/tracks/{id}/lyrics`.
pub fn lyrics_sign(track_id: &str) -> Result<(u64, String)> {
    let track_id = track_id.split(':').next().unwrap_or(track_id);
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| Error::Decode(format!("system clock is before Unix epoch: {error}")))?
        .as_secs();
    Ok((
        timestamp,
        lyrics_sign_at(track_id, timestamp, LYRICS_SIGN_KEY),
    ))
}

pub fn file_info_sign(
    track_id: &str,
    quality: &str,
    codecs: &[&str],
    transport: &str,
) -> Result<(u64, String)> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| Error::Decode(format!("system clock is before Unix epoch: {error}")))?
        .as_secs();
    Ok((
        timestamp,
        file_info_sign_at(
            track_id,
            quality,
            codecs,
            transport,
            timestamp,
            FILE_INFO_SIGN_KEY,
        ),
    ))
}

/// Preferred audio quality for track downloads.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioQuality {
    /// FLAC when available, MP3 320 otherwise.
    #[default]
    Lossless,
    /// MP3 320 kbps.
    High,
    /// Lower-bitrate MP3 for slower connections.
    Normal,
}

/// A track downloaded into memory, ready to be handed to an audio decoder.
#[derive(Debug, Clone)]
pub struct DownloadedTrack {
    /// Raw audio bytes (MP3 or FLAC).
    pub bytes: Vec<u8>,
    /// Signed URL the bytes were downloaded from.
    pub direct_url: String,
    /// Codec reported by the API.
    pub codec: String,
    /// Bitrate in kilobits per second.
    pub bitrate_kbps: u32,
}

/// Build a direct download URL from the XML document returned by Yandex.
pub fn build_direct_link(xml: &str) -> Result<String> {
    let host = extract_tag(xml, "host")
        .ok_or_else(|| Error::Decode("download info XML has no <host>".into()))?;
    let path = extract_tag(xml, "path")
        .ok_or_else(|| Error::Decode("download info XML has no <path>".into()))?;
    let ts = extract_tag(xml, "ts")
        .ok_or_else(|| Error::Decode("download info XML has no <ts>".into()))?;
    let signature = extract_tag(xml, "s")
        .ok_or_else(|| Error::Decode("download info XML has no <s>".into()))?;

    let path_without_slash = path.strip_prefix('/').unwrap_or(&path);
    let digest = Md5::digest(format!("{SIGN_SALT}{path_without_slash}{signature}").as_bytes());
    let sign = format!("{digest:x}");

    Ok(format!("https://{host}/get-mp3/{sign}/{ts}{path}"))
}

/// Pick an MP3 variant matching the requested quality.
pub fn select_mp3(infos: &[DownloadInfo], quality: AudioQuality) -> Option<&DownloadInfo> {
    let full = infos.iter().filter(|info| info.is_full_mp3());
    match quality {
        AudioQuality::Lossless | AudioQuality::High => full.max_by_key(|info| info.bitrate_in_kbps),
        AudioQuality::Normal => full
            .filter(|info| info.bitrate_in_kbps <= 192)
            .max_by_key(|info| info.bitrate_in_kbps)
            .or_else(|| {
                infos
                    .iter()
                    .filter(|info| !info.preview)
                    .min_by_key(|info| info.bitrate_in_kbps)
            }),
    }
    .or_else(|| select_best_mp3(infos))
}

/// Select the best full MP3 variant, preferring the highest bitrate.
pub fn select_best_mp3(infos: &[DownloadInfo]) -> Option<&DownloadInfo> {
    infos
        .iter()
        .filter(|info| info.is_full_mp3())
        .max_by_key(|info| info.bitrate_in_kbps)
        .or_else(|| {
            // Some responses omit `preview` or use a different codec label. Fall
            // back to the highest-bitrate entry rather than failing outright.
            infos
                .iter()
                .filter(|info| !info.preview)
                .max_by_key(|info| info.bitrate_in_kbps)
        })
}

/// Extract the text of the first `<tag>...</tag>` element.
///
/// The download XML is tiny and flat, so pulling in a full XML parser would be
/// overkill. Attributes are not supported because Yandex does not use them in
/// this document.
fn extract_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)?;
    let value = xml[start..start + end].trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const XML: &str = r#"<?xml version="1.0" encoding="utf-8"?>
        <download-info>
            <host>storage.mds.yandex.net</host>
            <path>/get-mp3/abc/123</path>
            <ts>1700000000</ts>
            <s>deadbeef</s>
        </download-info>"#;

    #[test]
    fn builds_signed_direct_link() {
        let url = build_direct_link(XML).unwrap();
        assert_eq!(
            url,
            "https://storage.mds.yandex.net/get-mp3/9e54247f3fb0e97968297c551424f282/1700000000/get-mp3/abc/123"
        );
    }

    #[test]
    fn rejects_incomplete_xml() {
        assert!(build_direct_link("<download-info><host>x</host></download-info>").is_err());
    }

    #[test]
    fn selects_highest_bitrate_full_mp3() {
        let infos = vec![
            DownloadInfo {
                codec: "mp3".into(),
                bitrate_in_kbps: 128,
                gain: false,
                preview: false,
                download_info_url: "a".into(),
                direct: false,
            },
            DownloadInfo {
                codec: "mp3".into(),
                bitrate_in_kbps: 320,
                gain: false,
                preview: false,
                download_info_url: "b".into(),
                direct: false,
            },
            DownloadInfo {
                codec: "mp3".into(),
                bitrate_in_kbps: 64,
                gain: false,
                preview: true,
                download_info_url: "c".into(),
                direct: false,
            },
        ];
        assert_eq!(select_best_mp3(&infos).unwrap().bitrate_in_kbps, 320);
    }

    #[test]
    fn falls_back_to_non_preview_codec() {
        let infos = vec![DownloadInfo {
            codec: "aac".into(),
            bitrate_in_kbps: 192,
            gain: false,
            preview: false,
            download_info_url: "a".into(),
            direct: false,
        }];
        assert_eq!(select_best_mp3(&infos).unwrap().bitrate_in_kbps, 192);
    }

    #[test]
    fn signs_file_info_with_desktop_key() {
        let sign = file_info_sign_at(
            "4784420:100500",
            "lossless",
            &["flac", "aac", "mp3"],
            "encraw",
            1668687184,
            "SUPER_SECRET_KEY",
        );
        assert_eq!(sign, "YH39qNGT90cOfpuP1sJgexVzcthU6SPLf9nSkbJnecQ");
        assert!(!sign.ends_with('='));
    }

    #[test]
    fn signs_lyrics_requests() {
        let sign = lyrics_sign_at("53404", 1668687184, LYRICS_SIGN_KEY);
        assert_eq!(sign, "Vdo8bB97WZwGaDNmBiUOBATOeQI6IeItEMAsHD0HSiA=");
    }

    #[test]
    fn parses_lossless_file_info_and_urls() {
        let json = r#"{
            "downloadInfo": {
                "quality": "lossless",
                "codec": "flac",
                "bitrate": 0,
                "transport": "raw",
                "url": "",
                "urls": ["https://cdn.example/song.flac"],
                "gain": false
            }
        }"#;
        let info: FileInfo = serde_json::from_str(json).unwrap();
        let file = info.playable().expect("playable flac");
        assert!(file.is_lossless());
        assert_eq!(file.direct_url(), Some("https://cdn.example/song.flac"));
    }

    #[test]
    fn rejects_encrypted_and_non_flac_mp4_variants() {
        let encrypted = FileDownloadInfo {
            codec: "flac".into(),
            transport: "encraw".into(),
            url: "https://cdn.example/song.flac".into(),
            ..Default::default()
        };
        assert!(!encrypted.is_playable());

        let aac_mp4 = FileDownloadInfo {
            codec: "aac-mp4".into(),
            transport: "raw".into(),
            url: "https://cdn.example/song.mp4".into(),
            ..Default::default()
        };
        assert!(!aac_mp4.is_playable());

        let flac_mp4 = FileDownloadInfo {
            codec: "flac-mp4".into(),
            transport: "raw".into(),
            url: "https://cdn.example/song.mp4".into(),
            ..Default::default()
        };
        assert!(flac_mp4.is_playable());
        assert!(flac_mp4.is_lossless());
    }
}
