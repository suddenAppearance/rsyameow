//! Track download information and direct link construction.
//!
//! The API returns a set of [`DownloadInfo`] entries. Each entry points to a
//! small XML document containing the storage host, the file path, a timestamp
//! and a signature. The direct MP3 URL is reconstructed exactly like the
//! official clients do:
//!
//! ```text
//! sign = md5(SALT + path.without_leading_slash() + s)
//! url  = https://{host}/get-mp3/{sign}/{ts}{path}
//! ```

use crate::{Error, Result};
use md5::{Digest, Md5};
use serde::Deserialize;

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

/// A track downloaded into memory, ready to be handed to an audio decoder.
#[derive(Debug, Clone)]
pub struct DownloadedTrack {
    /// Raw MP3 bytes.
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
}
