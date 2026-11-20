//! Data models returned by the Yandex Music API.
//!
//! Yandex responds with `camelCase` field names and occasionally encodes numeric
//! identifiers as strings. The models below are deliberately tolerant: almost
//! every field is optional, unknown fields are ignored and missing arrays are
//! replaced by empty vectors.

use serde::{Deserialize, Deserializer};
use std::time::Duration;

/// A number that can arrive either as a JSON string or as a JSON number.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum StringOrNumber {
    String(String),
    Signed(i64),
    Unsigned(u64),
}

impl StringOrNumber {
    fn into_string(self) -> String {
        match self {
            Self::String(value) => value,
            Self::Signed(value) => value.to_string(),
            Self::Unsigned(value) => value.to_string(),
        }
    }
}

pub(crate) fn de_string_or_number<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(StringOrNumber::deserialize(deserializer)?.into_string())
}

pub(crate) fn de_opt_string_or_number<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<StringOrNumber>::deserialize(deserializer)?.map(StringOrNumber::into_string))
}

pub(crate) fn de_opt_u64<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Option::<StringOrNumber>::deserialize(deserializer)?
        .and_then(|value| value.into_string().parse().ok()))
}

/// Build a cover URL from a Yandex cover URI.
///
/// Cover URIs contain a `%%` placeholder that is replaced by a size such as
/// `400x400`. When the placeholder is absent the URI is returned unchanged.
pub fn cover_url(uri: &str, size: u32) -> String {
    let uri = uri.replace("%%", &format!("{size}x{size}"));
    if uri.starts_with("http://") || uri.starts_with("https://") {
        uri
    } else {
        format!("https://{uri}")
    }
}

/// Basic account information.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    /// Server time at the moment of the request.
    #[serde(default)]
    pub now: Option<String>,
    /// Unique user id.
    #[serde(default)]
    pub uid: Option<u64>,
    /// Login, usually an email address.
    #[serde(default)]
    pub login: Option<String>,
    #[serde(default)]
    pub full_name: Option<String>,
    #[serde(default)]
    pub first_name: Option<String>,
    #[serde(default)]
    pub second_name: Option<String>,
    /// Name chosen for display in the service.
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub service_available: Option<bool>,
    #[serde(default)]
    pub hosted_user: Option<bool>,
    #[serde(default)]
    pub birthday: Option<String>,
    #[serde(default)]
    pub region: Option<u64>,
    #[serde(default)]
    pub region_code: Option<String>,
    #[serde(default)]
    pub registered_at: Option<String>,
    #[serde(default)]
    pub child: Option<bool>,
    #[serde(default)]
    pub has_info_for_app_metrica: Option<bool>,
    #[serde(default)]
    pub non_owner_family_member: Option<bool>,
}

impl Account {
    /// Best human-readable name for the account.
    pub fn best_name(&self) -> &str {
        self.display_name
            .as_deref()
            .filter(|name| !name.trim().is_empty())
            .or(self.full_name.as_deref())
            .or(self.login.as_deref())
            .unwrap_or("Yandex Music user")
    }
}

/// User permissions, for example an active subscription.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Permissions {
    #[serde(default)]
    pub until: Option<String>,
    #[serde(default)]
    pub values: Vec<String>,
    #[serde(default)]
    pub default: Vec<String>,
}

/// Yandex Plus subscription information.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Plus {
    #[serde(default)]
    pub has_plus: bool,
    #[serde(default)]
    pub is_tutorial_completed: bool,
    #[serde(default)]
    pub migrated: Option<bool>,
}

/// Subscription information (the subset used by the application).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Subscription {
    #[serde(default)]
    pub end: Option<String>,
    #[serde(default)]
    pub can_start_trial: Option<bool>,
    #[serde(default)]
    pub had_any_subscription: Option<bool>,
    #[serde(default)]
    pub non_auto_renewable: Option<serde_json::Value>,
    #[serde(default)]
    pub auto_renewable: Vec<serde_json::Value>,
    #[serde(default)]
    pub family_auto_renewable: Vec<serde_json::Value>,
}

/// Result of `GET /account/status`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    #[serde(default)]
    pub account: Option<Account>,
    #[serde(default)]
    pub permissions: Option<Permissions>,
    #[serde(default)]
    pub subscription: Option<Subscription>,
    #[serde(default)]
    pub plus: Option<Plus>,
    #[serde(default)]
    pub advertisement: Option<String>,
    #[serde(default)]
    pub cache_limit: Option<u64>,
    #[serde(default)]
    pub subeditor: Option<bool>,
    #[serde(default)]
    pub subeditor_level: Option<u64>,
    #[serde(default)]
    pub default_email: Option<String>,
    #[serde(default)]
    pub skips_per_hour: Option<u64>,
    #[serde(default)]
    pub station_exists: Option<bool>,
    #[serde(default)]
    pub premium_region: Option<u64>,
    #[serde(default)]
    pub experiment: Option<u64>,
    #[serde(default)]
    pub pretrial_active: Option<bool>,
    #[serde(default)]
    pub userhash: Option<String>,
    #[serde(default)]
    pub has_options: Vec<String>,
}

impl Status {
    /// Returns `true` when the account has an active Plus subscription.
    pub fn has_plus(&self) -> bool {
        self.plus.as_ref().is_some_and(|plus| plus.has_plus)
    }
}

/// An artist as embedded into a track or an album.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artist {
    #[serde(default, deserialize_with = "de_opt_u64")]
    pub id: Option<u64>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub cover: Option<Cover>,
    #[serde(default)]
    pub og_image: Option<String>,
    #[serde(default)]
    pub op_image: Option<String>,
    #[serde(default)]
    pub genres: Vec<String>,
    #[serde(default)]
    pub likes_count: Option<u64>,
    #[serde(default)]
    pub counts: Option<ArtistCounts>,
    #[serde(default)]
    pub various: Option<bool>,
    #[serde(default)]
    pub composer: Option<bool>,
}

/// Track and album counters shown on an artist page.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistCounts {
    #[serde(default)]
    pub tracks: u64,
    #[serde(default)]
    pub direct_albums: u64,
    #[serde(default)]
    pub also_albums: u64,
    #[serde(default)]
    pub also_tracks: u64,
}

impl Artist {
    /// Artist name with a fallback for malformed search results.
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or("Неизвестный исполнитель")
    }

    /// URL of the artist cover at the requested size.
    pub fn cover_url(&self, size: u32) -> Option<String> {
        self.cover
            .as_ref()
            .and_then(|cover| cover.url(size))
            .or_else(|| {
                self.op_image
                    .as_deref()
                    .filter(|uri| !uri.is_empty())
                    .map(|uri| cover_url(uri, size))
            })
            .or_else(|| {
                self.og_image
                    .as_deref()
                    .filter(|uri| !uri.is_empty())
                    .map(|uri| cover_url(uri, size))
            })
    }
}

/// An album as embedded into a track.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Album {
    #[serde(default, deserialize_with = "de_opt_u64")]
    pub id: Option<u64>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub year: Option<u64>,
    #[serde(default)]
    pub cover_uri: Option<String>,
    #[serde(default)]
    pub track_count: Option<u64>,
    #[serde(default)]
    pub genre: Option<String>,
    #[serde(default)]
    pub release_date: Option<String>,
    #[serde(default)]
    pub artists: Vec<Artist>,
    #[serde(default)]
    pub volumes: Vec<Vec<Track>>,
    #[serde(default)]
    pub likes_count: Option<u64>,
    #[serde(default)]
    pub short_description: Option<String>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub pager: Option<Pager>,
}

impl Album {
    /// Album title with a fallback.
    pub fn display_title(&self) -> &str {
        self.title.as_deref().unwrap_or("Без названия")
    }

    /// Comma-separated artist names.
    pub fn artist_names(&self) -> String {
        self.artists
            .iter()
            .filter_map(|artist| artist.name.as_deref())
            .filter(|name| !name.is_empty())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// URL of the album cover at the requested size.
    pub fn cover_url(&self, size: u32) -> Option<String> {
        self.cover_uri
            .as_deref()
            .filter(|uri| !uri.is_empty())
            .map(|uri| cover_url(uri, size))
    }

    /// All tracks of the album across its volumes.
    pub fn all_tracks(&self) -> Vec<Track> {
        self.volumes.iter().flatten().cloned().collect()
    }

    /// Total duration of the album.
    pub fn duration(&self) -> Duration {
        Duration::from_millis(
            self.duration_ms
                .or_else(|| {
                    let milliseconds = self
                        .all_tracks()
                        .iter()
                        .map(|track| track.duration_ms.unwrap_or(0))
                        .sum::<u64>();
                    (milliseconds > 0).then_some(milliseconds)
                })
                .unwrap_or(0),
        )
    }
}

/// Pager metadata returned by artist lists.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pager {
    #[serde(default)]
    pub total: u64,
    #[serde(default)]
    pub page: u64,
    #[serde(default)]
    pub per_page: u64,
}

/// A batch of tracks from a rotor/wave station.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaveBatch {
    #[serde(default)]
    pub batch_id: Option<String>,
    #[serde(default)]
    pub radio_session_id: Option<String>,
    #[serde(default)]
    pub sequence: Vec<WaveSequence>,
}

impl WaveBatch {
    /// Playable tracks from the batch in station order.
    pub fn tracks(&self) -> Vec<Track> {
        self.sequence
            .iter()
            .filter_map(|item| item.track.clone())
            .collect()
    }
}

/// One entry in a wave sequence.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaveSequence {
    #[serde(rename = "type", default)]
    pub kind: String,
    #[serde(default)]
    pub track: Option<Track>,
    #[serde(default)]
    pub liked: bool,
}

/// A page of artist tracks.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistTracks {
    #[serde(default)]
    pub tracks: Vec<Track>,
    #[serde(default)]
    pub pager: Option<Pager>,
}

/// A liked album entry from the user's collection.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LikedAlbum {
    #[serde(default)]
    pub id: u64,
}

/// A page of artist albums.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtistAlbums {
    #[serde(default)]
    pub albums: Vec<Album>,
    #[serde(default)]
    pub pager: Option<Pager>,
}

/// A cover object (artists expose covers as objects rather than URIs).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cover {
    #[serde(default)]
    pub uri: Option<String>,
    #[serde(default)]
    pub items_uri: Vec<String>,
    #[serde(default)]
    pub color: Option<String>,
}

impl Cover {
    /// Resolve the cover to an absolute URL.
    pub fn url(&self, size: u32) -> Option<String> {
        self.uri
            .as_deref()
            .filter(|uri| !uri.is_empty())
            .or_else(|| {
                self.items_uri
                    .iter()
                    .find(|uri| !uri.is_empty())
                    .map(String::as_str)
            })
            .map(|uri| cover_url(uri, size))
    }
}

/// A music track.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    #[serde(deserialize_with = "de_string_or_number")]
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub available: Option<bool>,
    #[serde(default)]
    pub artists: Vec<Artist>,
    #[serde(default)]
    pub albums: Vec<Album>,
    #[serde(default)]
    pub cover_uri: Option<String>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub explicit: Option<bool>,
    #[serde(default)]
    pub available_for_premium_users: Option<bool>,
    #[serde(default)]
    pub preview_duration_ms: Option<u64>,
    #[serde(default)]
    pub short_description: Option<String>,
    /// Some podcast tracks use `podcast-episode` here.
    #[serde(rename = "type", default)]
    pub track_type: Option<String>,
}

impl Track {
    /// Comma-separated list of artist names.
    pub fn artist_names(&self) -> String {
        self.artists
            .iter()
            .filter_map(|artist| artist.name.as_deref())
            .filter(|name| !name.is_empty())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Track duration. Falls back to zero when the API did not send it.
    pub fn duration(&self) -> Duration {
        Duration::from_millis(self.duration_ms.unwrap_or(0))
    }

    /// Track title with a fallback for malformed responses.
    pub fn display_title(&self) -> &str {
        self.title.as_deref().unwrap_or("Without title")
    }

    /// URL of the track cover at the requested size.
    pub fn cover_url(&self, size: u32) -> Option<String> {
        self.cover_uri
            .as_deref()
            .filter(|uri| !uri.is_empty())
            .map(|uri| cover_url(uri, size))
            .or_else(|| {
                self.albums
                    .iter()
                    .find_map(|album| album.cover_uri.as_deref())
                    .filter(|uri| !uri.is_empty())
                    .map(|uri| cover_url(uri, size))
            })
    }

    /// Full track id in the `id:album_id` format used by some endpoints.
    pub fn full_id(&self) -> String {
        match self.albums.first().and_then(|album| album.id) {
            Some(album_id) => format!("{}:{}", self.id, album_id),
            None => self.id.clone(),
        }
    }

    /// Whether the track can be streamed.
    pub fn is_playable(&self) -> bool {
        self.available.unwrap_or(true)
    }
}

/// Lyric metadata returned by `/tracks/{id}/lyrics`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackLyrics {
    #[serde(default)]
    pub download_url: Option<String>,
    #[serde(default)]
    pub lyric_id: Option<u64>,
    #[serde(default)]
    pub external_lyric_id: Option<String>,
    #[serde(default)]
    pub writers: Vec<String>,
    #[serde(default)]
    pub major: Option<LyricsMajor>,
}

/// The lyrics provider (Musixmatch and similar).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricsMajor {
    #[serde(default)]
    pub id: Option<u64>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub pretty_name: Option<String>,
}

/// A lightweight track reference used in like lists and playlists.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackShort {
    #[serde(deserialize_with = "de_string_or_number")]
    pub id: String,
    #[serde(default, deserialize_with = "de_opt_string_or_number")]
    pub album_id: Option<String>,
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(default)]
    pub play_count: Option<u64>,
    #[serde(default)]
    pub recent: Option<bool>,
    #[serde(default)]
    pub original_index: Option<u64>,
    /// Sometimes the full track is embedded in the short representation.
    #[serde(default)]
    pub track: Option<Track>,
}

impl TrackShort {
    /// Id in the `id:album_id` format expected by all track endpoints.
    pub fn track_id(&self) -> String {
        match self.album_id.as_deref().filter(|album| !album.is_empty()) {
            Some(album_id) => format!("{}:{}", self.id, album_id),
            None => self.id.clone(),
        }
    }
}

/// A paginated block of search results.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult<T> {
    #[serde(default)]
    pub total: u64,
    #[serde(default)]
    pub per_page: u64,
    #[serde(default)]
    pub order: u64,
    #[serde(default)]
    pub results: Vec<T>,
}

/// A Yandex Music user (playlist owner, search result, ...).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    #[serde(default, deserialize_with = "de_opt_u64")]
    pub uid: Option<u64>,
    #[serde(default)]
    pub login: Option<String>,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub full_name: Option<String>,
}

/// A user playlist. The `/playlists/list` endpoint returns playlists without
/// tracks, while `/users/{uid}/playlists/{kind}` includes them.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    #[serde(default)]
    pub uid: Option<u64>,
    #[serde(default)]
    pub kind: Option<u64>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub track_count: Option<u64>,
    #[serde(default)]
    pub visibility: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub cover: Option<Cover>,
    #[serde(default)]
    pub og_image: Option<String>,
    #[serde(default)]
    pub owner: Option<User>,
    #[serde(default)]
    pub tracks: Vec<TrackShort>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub generated_playlist_type: Option<String>,
    #[serde(default)]
    pub created: Option<String>,
    #[serde(default)]
    pub modified: Option<String>,
    #[serde(default)]
    pub snapshot: Option<u64>,
    #[serde(default)]
    pub playlist_uuid: Option<String>,
}

impl Playlist {
    /// Unique id in the `uid:kind` format.
    pub fn playlist_id(&self) -> String {
        match (self.uid, self.kind) {
            (Some(uid), Some(kind)) => format!("{uid}:{kind}"),
            _ => String::new(),
        }
    }

    /// Playlist title with a fallback.
    pub fn display_title(&self) -> &str {
        self.title
            .as_deref()
            .filter(|title| !title.trim().is_empty())
            .unwrap_or("Без названия")
    }

    /// URL of the playlist cover at the requested size.
    pub fn cover_url(&self, size: u32) -> Option<String> {
        self.cover
            .as_ref()
            .and_then(|cover| cover.url(size))
            .or_else(|| {
                self.og_image
                    .as_deref()
                    .filter(|uri| !uri.is_empty())
                    .map(|uri| cover_url(uri, size))
            })
    }

    /// Playlist duration.
    pub fn duration(&self) -> Duration {
        Duration::from_millis(self.duration_ms.unwrap_or(0))
    }

    /// Whether the playlist is visible to everyone.
    pub fn is_public(&self) -> bool {
        self.visibility.as_deref() == Some("public")
    }
}

/// The user's library of liked tracks.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TracksList {
    #[serde(default)]
    pub uid: Option<u64>,
    #[serde(default)]
    pub revision: Option<i64>,
    #[serde(default)]
    pub tracks: Vec<TrackShort>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_track_with_numeric_and_string_ids() {
        let track: Track = serde_json::from_str(
            r#"{
                "id": 123,
                "title": "Test",
                "artists": [{"id": "42", "name": "Artist"}],
                "albums": [{"id": 7, "title": "Album", "coverUri": "avatars.yandex.net/x/%%"}],
                "durationMs": 61000,
                "coverUri": "avatars.yandex.net/y/%%"
            }"#,
        )
        .unwrap();

        assert_eq!(track.id, "123");
        assert_eq!(track.artist_names(), "Artist");
        assert_eq!(track.duration(), Duration::from_secs(61));
        assert_eq!(track.full_id(), "123:7");
        assert_eq!(
            track.cover_url(400).unwrap(),
            "https://avatars.yandex.net/y/400x400"
        );
    }

    #[test]
    fn cover_url_handles_absolute_urls_and_missing_placeholder() {
        assert_eq!(
            cover_url("https://example.com/a.jpg", 100),
            "https://example.com/a.jpg"
        );
        assert_eq!(
            cover_url("avatars.yandex.net/a/%%", 200),
            "https://avatars.yandex.net/a/200x200"
        );
    }

    #[test]
    fn account_best_name_falls_back() {
        let account = Account {
            display_name: Some("   ".into()),
            full_name: Some("Full Name".into()),
            login: Some("login".into()),
            ..Default::default()
        };
        assert_eq!(account.best_name(), "Full Name");

        let account = Account {
            login: Some("login".into()),
            ..Default::default()
        };
        assert_eq!(account.best_name(), "login");
    }

    #[test]
    fn status_plus_detection() {
        let status: Status = serde_json::from_str(r#"{"plus":{"hasPlus":true}}"#).unwrap();
        assert!(status.has_plus());

        let status: Status = serde_json::from_str(r#"{"plus":null}"#).unwrap();
        assert!(!status.has_plus());
    }
}
