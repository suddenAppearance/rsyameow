//! The Yandex Music API client.

use crate::auth::{
    DEFAULT_CLIENT_ID, DEFAULT_CLIENT_SECRET, DEFAULT_DEVICE_NAME, DeviceCode, OAuthToken,
};
use crate::download::{
    DownloadInfo, DownloadedTrack, FILE_INFO_CLIENT, FILE_INFO_CODECS, FileInfo, build_direct_link,
    file_info_sign, lyrics_sign, select_best_mp3,
};
use crate::http::Transport;
use crate::models::{
    Album, Artist, ArtistAlbums, ArtistTracks, Playlist, SearchResult, Status, Track, TrackLyrics,
    TracksList, WaveBatch,
};
use crate::{Error, Result};
use serde::Deserialize;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DEFAULT_BASE_URL: &str = "https://api.music.yandex.net";
const DEFAULT_OAUTH_BASE_URL: &str = "https://oauth.yandex.ru";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Builder for [`Client`].
#[derive(Debug, Clone)]
pub struct ClientBuilder {
    token: Option<String>,
    base_url: String,
    oauth_base_url: String,
    timeout: Duration,
    client_id: String,
    client_secret: String,
    device_name: String,
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self {
            token: None,
            base_url: DEFAULT_BASE_URL.to_string(),
            oauth_base_url: DEFAULT_OAUTH_BASE_URL.to_string(),
            timeout: DEFAULT_TIMEOUT,
            client_id: DEFAULT_CLIENT_ID.to_string(),
            client_secret: DEFAULT_CLIENT_SECRET.to_string(),
            device_name: DEFAULT_DEVICE_NAME.to_string(),
        }
    }
}

impl ClientBuilder {
    /// Create a builder with production Yandex endpoints and no token.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set an existing OAuth token.
    pub fn token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Override the API base URL (useful for tests and proxies).
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into().trim_end_matches('/').to_string();
        self
    }

    /// Override the OAuth base URL.
    pub fn oauth_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.oauth_base_url = base_url.into().trim_end_matches('/').to_string();
        self
    }

    /// Set the request timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Override the OAuth client id (the default is the public Android app id).
    pub fn client_id(mut self, client_id: impl Into<String>) -> Self {
        self.client_id = client_id.into();
        self
    }

    /// Override the OAuth client secret.
    pub fn client_secret(mut self, client_secret: impl Into<String>) -> Self {
        self.client_secret = client_secret.into();
        self
    }

    /// Set the device name shown on the Yandex confirmation page.
    pub fn device_name(mut self, device_name: impl Into<String>) -> Self {
        self.device_name = device_name.into();
        self
    }

    /// Finish building the client.
    pub fn build(self) -> Client {
        Client {
            inner: Arc::new(ClientInner {
                http: Transport::new(self.timeout),
                base_url: self.base_url,
                oauth_base_url: self.oauth_base_url,
                token: RwLock::new(self.token),
                uid: RwLock::new(None),
                client_id: self.client_id,
                client_secret: self.client_secret,
                device_name: self.device_name,
            }),
        }
    }
}

struct ClientInner {
    http: Transport,
    base_url: String,
    oauth_base_url: String,
    token: RwLock<Option<String>>,
    uid: RwLock<Option<u64>>,
    client_id: String,
    client_secret: String,
    device_name: String,
}

/// A cheaply cloneable, thread-safe Yandex Music API client.
///
/// The client is synchronous; call it from a worker thread (or from
/// `gpui`'s background executor) to keep the UI responsive.
#[derive(Clone)]
pub struct Client {
    inner: Arc<ClientInner>,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Client")
            .field("authenticated", &self.token().is_some())
            .field("cached_uid", &self.cached_uid())
            .finish()
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::unauthenticated()
    }
}

impl Client {
    /// Start configuring a client.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    /// Create a client without a token.
    pub fn unauthenticated() -> Self {
        ClientBuilder::new().build()
    }

    /// Create an authenticated client.
    pub fn with_token(token: impl Into<String>) -> Self {
        ClientBuilder::new().token(token).build()
    }

    /// The current token, if any.
    pub fn token(&self) -> Option<String> {
        self.inner
            .token
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Replace the token. The cached account id is reset.
    pub fn set_token(&self, token: impl Into<String>) {
        self.set_token_opt(Some(token.into()));
    }

    /// Set or clear the token.
    pub fn set_token_opt(&self, token: Option<String>) {
        *self
            .inner
            .token
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = token;
        *self
            .inner
            .uid
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
    }

    /// Forget the token and the cached account id.
    pub fn clear_token(&self) {
        self.set_token_opt(None);
    }

    /// The account id cached by [`Client::init`].
    pub fn cached_uid(&self) -> Option<u64> {
        *self
            .inner
            .uid
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn require_token(&self) -> Result<String> {
        self.token().ok_or(Error::NotAuthenticated)
    }

    fn api_url(&self, path: &str) -> String {
        format!("{}{}", self.inner.base_url, path)
    }

    fn oauth_url(&self, path: &str) -> String {
        format!("{}{}", self.inner.oauth_base_url, path)
    }

    // ---------------------------------------------------------------- OAuth

    /// Request a device code for the OAuth device authorization flow.
    ///
    /// Show [`DeviceCode::verification_url`] and [`DeviceCode::user_code`] to
    /// the user, then poll [`Client::poll_device_token`] with the returned
    /// `device_code`.
    pub fn request_device_code(&self) -> Result<DeviceCode> {
        let form = vec![
            ("client_id", self.inner.client_id.clone()),
            ("device_id", random_device_id()),
            ("device_name", self.inner.device_name.clone()),
        ];
        let url = self.oauth_url("/device/code");
        self.inner
            .http
            .post_form_flat(&url, &form)
            .map_err(|error| match error {
                Error::BadRequest(message) => Error::DeviceAuth(message),
                other => other,
            })
    }

    /// Poll the OAuth server once for an access token.
    ///
    /// Returns `Ok(None)` while the user has not confirmed the code yet
    /// (`authorization_pending`). Every other OAuth error is returned as
    /// [`Error::DeviceAuth`].
    pub fn poll_device_token(&self, device_code: &str) -> Result<Option<OAuthToken>> {
        let form = vec![
            ("grant_type", "device_code".to_string()),
            ("code", device_code.to_string()),
            ("client_id", self.inner.client_id.clone()),
            ("client_secret", self.inner.client_secret.clone()),
        ];
        let url = self.oauth_url("/token");

        match self.inner.http.post_form_flat(&url, &form) {
            Ok(token) => Ok(Some(token)),
            Err(Error::BadRequest(message)) if message.contains("authorization_pending") => {
                Ok(None)
            }
            Err(Error::BadRequest(message)) => Err(Error::DeviceAuth(message)),
            Err(Error::Api { status: _, message }) if message.contains("authorization_pending") => {
                Ok(None)
            }
            Err(Error::Api { status, message }) => {
                Err(Error::DeviceAuth(format!("HTTP {status}: {message}")))
            }
            Err(Error::Unauthorized(message)) => Err(Error::DeviceAuth(message)),
            Err(error) => Err(error),
        }
    }

    // ------------------------------------------------------------- Playlists

    /// Playlists of the current account.
    pub fn playlists(&self) -> Result<Vec<Playlist>> {
        let uid = self.require_uid()?;
        self.user_playlists(uid)
    }

    /// Playlists of an arbitrary account (short representation without tracks).
    pub fn user_playlists(&self, uid: u64) -> Result<Vec<Playlist>> {
        let token = self.require_token()?;
        let url = self.api_url(&format!("/users/{uid}/playlists/list"));
        let playlists: Option<Vec<Playlist>> = self.inner.http.get(&url, &[], Some(&token))?;
        Ok(playlists.unwrap_or_default())
    }

    /// A single playlist with its tracks.
    pub fn user_playlist(&self, uid: u64, kind: u64) -> Result<Playlist> {
        let token = self.require_token()?;
        let url = self.api_url(&format!("/users/{uid}/playlists/{kind}"));
        let playlist: Option<Playlist> = self.inner.http.get(&url, &[], Some(&token))?;
        playlist.ok_or_else(|| Error::NotFound(format!("playlist {uid}:{kind}")))
    }

    // -------------------------------------------------------------- Account

    /// Fetch the full account status.
    pub fn account_status(&self) -> Result<Status> {
        let token = self.require_token()?;
        let url = self.api_url("/account/status");
        self.inner.http.get(&url, &[], Some(&token))
    }

    /// Fetch the account status and remember the account id for subsequent
    /// calls such as [`Client::liked_tracks`].
    pub fn init(&self) -> Result<Status> {
        let status = self.account_status()?;
        let uid = status
            .account
            .as_ref()
            .and_then(|account| account.uid)
            .ok_or_else(|| Error::Decode("account status contains no uid".into()))?;
        *self
            .inner
            .uid
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(uid);
        Ok(status)
    }

    // --------------------------------------------------------------- Tracks

    /// Fetch full tracks by their ids (`id` or `id:album_id`).
    pub fn tracks<I, S>(&self, ids: I) -> Result<Vec<Track>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let ids: Vec<String> = ids
            .into_iter()
            .map(|id| id.as_ref().to_string())
            .filter(|id| !id.is_empty())
            .collect();
        if ids.is_empty() {
            return Ok(Vec::new());
        }

        let token = self.require_token()?;
        let url = self.api_url("/tracks");
        let mut form = Vec::with_capacity(ids.len() + 1);
        form.push(("with-positions", "true".to_string()));
        for id in &ids {
            form.push(("track-ids", id.clone()));
        }

        let tracks: Vec<Track> = self.inner.http.post_form(&url, &form, Some(&token))?;
        Ok(tracks)
    }

    /// Fetch one track.
    pub fn track(&self, id: &str) -> Result<Track> {
        self.tracks([id])?
            .into_iter()
            .next()
            .ok_or_else(|| Error::NotFound(format!("track {id}")))
    }

    // ---------------------------------------------------------------- Search

    /// Search for tracks by title, artist or album.
    ///
    /// `page` starts at zero. Yandex usually returns 20 tracks per page.
    pub fn search_tracks(&self, text: &str, page: u32) -> Result<SearchResult<Track>> {
        let token = self.require_token()?;
        let text = text.trim();
        if text.is_empty() {
            return Err(Error::BadRequest("search query is empty".into()));
        }

        let url = self.api_url("/search");
        let query = [
            ("text", text.to_string()),
            ("nocorrect", "false".to_string()),
            ("type", "track".to_string()),
            ("page", page.to_string()),
            ("playlist-in-best", "true".to_string()),
        ];

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct SearchResponse {
            #[serde(default)]
            tracks: Option<SearchResult<Track>>,
        }

        let response: SearchResponse = self.inner.http.get(&url, &query, Some(&token))?;
        Ok(response.tracks.unwrap_or_default())
    }

    /// Search for artists.
    pub fn search_artists(&self, text: &str, page: u32) -> Result<SearchResult<Artist>> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct SearchResponse {
            #[serde(default)]
            artists: Option<SearchResult<Artist>>,
        }

        let value = self.search_raw(text, page, "artist")?;
        let response: SearchResponse = serde_json::from_value(value)
            .map_err(|error| Error::Decode(format!("search artists: {error}")))?;
        Ok(response.artists.unwrap_or_default())
    }

    /// Search for albums.
    pub fn search_albums(&self, text: &str, page: u32) -> Result<SearchResult<Album>> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct SearchResponse {
            #[serde(default)]
            albums: Option<SearchResult<Album>>,
        }

        let value = self.search_raw(text, page, "album")?;
        let response: SearchResponse = serde_json::from_value(value)
            .map_err(|error| Error::Decode(format!("search albums: {error}")))?;
        Ok(response.albums.unwrap_or_default())
    }

    fn search_raw(&self, text: &str, page: u32, kind: &str) -> Result<serde_json::Value> {
        let token = self.require_token()?;
        let text = text.trim();
        if text.is_empty() {
            return Err(Error::BadRequest("search query is empty".into()));
        }

        let url = self.api_url("/search");
        let query = [
            ("text", text.to_string()),
            ("nocorrect", "false".to_string()),
            ("type", kind.to_string()),
            ("page", page.to_string()),
            ("playlist-in-best", "true".to_string()),
        ];

        self.inner.http.get(&url, &query, Some(&token))
    }

    // ---------------------------------------------------------------- Likes

    fn require_uid(&self) -> Result<u64> {
        if let Some(uid) = self.cached_uid() {
            return Ok(uid);
        }
        let status = self.init()?;
        status
            .account
            .and_then(|account| account.uid)
            .ok_or_else(|| Error::Decode("account status contains no uid".into()))
    }

    /// Like tracks.
    ///
    /// Calls [`Client::init`] automatically when the account id is not cached.
    pub fn like_tracks(&self, track_ids: &[String]) -> Result<()> {
        self.change_track_likes(track_ids, true)
    }

    /// Remove tracks from the likes.
    pub fn unlike_tracks(&self, track_ids: &[String]) -> Result<()> {
        self.change_track_likes(track_ids, false)
    }

    fn change_track_likes(&self, track_ids: &[String], liked: bool) -> Result<()> {
        if track_ids.is_empty() {
            return Ok(());
        }

        let token = self.require_token()?;
        let uid = self.require_uid()?;
        let action = if liked { "add-multiple" } else { "remove" };
        let url = self.api_url(&format!("/users/{uid}/likes/tracks/{action}"));
        let form: Vec<(&str, String)> = track_ids
            .iter()
            .map(|id| ("track-ids", id.clone()))
            .collect();

        let _: serde_json::Value = self.inner.http.post_form(&url, &form, Some(&token))?;
        Ok(())
    }

    /// Liked tracks of the current account.
    ///
    /// Calls [`Client::init`] automatically when the account id is not cached.
    pub fn liked_tracks(&self) -> Result<TracksList> {
        let uid = match self.cached_uid() {
            Some(uid) => uid,
            None => self
                .init()?
                .account
                .and_then(|account| account.uid)
                .ok_or(Error::Decode("account status contains no uid".into()))?,
        };
        self.liked_tracks_for(uid)
    }

    /// Liked tracks of an arbitrary account.
    pub fn liked_tracks_for(&self, uid: u64) -> Result<TracksList> {
        let token = self.require_token()?;
        let url = self.api_url(&format!("/users/{uid}/likes/tracks"));
        let query = [("if-modified-since-revision", "0".to_string())];

        #[derive(Deserialize)]
        struct LikesResponse {
            #[serde(default)]
            library: Option<TracksList>,
        }

        let response: LikesResponse = self.inner.http.get(&url, &query, Some(&token))?;
        Ok(response.library.unwrap_or_default())
    }

    // ------------------------------------------------------------------- Wave

    /// A batch of tracks from a rotor station. `queue` is the id of the last
    /// played track and asks the station to continue from it.
    pub fn rotor_station_tracks(&self, station: &str, queue: Option<&str>) -> Result<WaveBatch> {
        let token = self.require_token()?;
        let url = self.api_url(&format!("/rotor/station/{station}/tracks"));
        let mut query = vec![("settings2", "true".to_string())];
        if let Some(queue) = queue.filter(|queue| !queue.is_empty()) {
            query.push(("queue", queue.to_string()));
        }

        let batch: Option<WaveBatch> = self.inner.http.get(&url, &query, Some(&token))?;
        batch.ok_or_else(|| Error::NotFound(format!("station {station}")))
    }

    /// A batch of "Моя волна" (`user:onyourwave`).
    pub fn wave_tracks(&self, queue: Option<&str>) -> Result<WaveBatch> {
        self.rotor_station_tracks("user:onyourwave", queue)
    }

    // ---------------------------------------------------------- Artist / Album

    /// Artist information from the brief-info endpoint.
    pub fn artist(&self, artist_id: u64) -> Result<Artist> {
        let token = self.require_token()?;
        let url = self.api_url(&format!("/artists/{artist_id}/brief-info"));

        #[derive(Deserialize)]
        struct BriefInfo {
            #[serde(default)]
            artist: Option<Artist>,
        }

        let info: BriefInfo = self.inner.http.get(&url, &[], Some(&token))?;
        info.artist
            .ok_or_else(|| Error::NotFound(format!("artist {artist_id}")))
    }

    /// A page of an artist's tracks.
    pub fn artist_tracks(&self, artist_id: u64, page: u32, page_size: u32) -> Result<ArtistTracks> {
        let token = self.require_token()?;
        let url = self.api_url(&format!("/artists/{artist_id}/tracks"));
        let query = [
            ("page", page.to_string()),
            ("page-size", page_size.to_string()),
        ];
        self.inner.http.get(&url, &query, Some(&token))
    }

    /// A page of an artist's direct albums.
    pub fn artist_albums(&self, artist_id: u64, page: u32, page_size: u32) -> Result<ArtistAlbums> {
        let token = self.require_token()?;
        let url = self.api_url(&format!("/artists/{artist_id}/direct-albums"));
        let query = [
            ("sort-by", "year".to_string()),
            ("page", page.to_string()),
            ("page-size", page_size.to_string()),
        ];
        self.inner.http.get(&url, &query, Some(&token))
    }

    /// An album with its tracks grouped into volumes.
    pub fn album_with_tracks(&self, album_id: u64) -> Result<Album> {
        let token = self.require_token()?;
        let url = self.api_url(&format!("/albums/{album_id}/with-tracks"));
        let album: Option<Album> = self.inner.http.get(&url, &[], Some(&token))?;
        album.ok_or_else(|| Error::NotFound(format!("album {album_id}")))
    }

    // ------------------------------------------------------------- Download

    /// Available download variants for a track.
    pub fn download_info(&self, track_id: &str) -> Result<Vec<DownloadInfo>> {
        let token = self.require_token()?;
        let url = self.api_url(&format!("/tracks/{track_id}/download-info"));
        let infos: Option<Vec<DownloadInfo>> = self.inner.http.get(&url, &[], Some(&token))?;
        Ok(infos.unwrap_or_default())
    }

    /// Resolve the signed direct MP3 URL for a track.
    pub fn direct_link(&self, track_id: &str) -> Result<String> {
        let infos = self.download_info(track_id)?;
        let info = select_best_mp3(&infos).ok_or(Error::NoDownload)?;
        let token = self.require_token()?;
        let xml = self
            .inner
            .http
            .get_text(&info.download_info_url, Some(&token))?;
        build_direct_link(&xml)
    }

    /// Signed file information for a track. Prefers raw FLAC and falls back to
    /// MP3 when lossless is unavailable.
    pub fn file_info(&self, track_id: &str) -> Result<FileInfo> {
        let token = self.require_token()?;
        let (timestamp, sign) = file_info_sign(track_id, "lossless", FILE_INFO_CODECS, "raw")?;
        let url = self.api_url("/get-file-info");
        let query = [
            ("ts", timestamp.to_string()),
            ("trackId", track_id.to_string()),
            ("quality", "lossless".to_string()),
            ("codecs", FILE_INFO_CODECS.join(",")),
            ("transports", "raw".to_string()),
            ("sign", sign),
        ];

        let info: Option<FileInfo> =
            self.inner
                .http
                .get_with_client(&url, &query, Some(&token), FILE_INFO_CLIENT)?;
        info.ok_or(Error::NoDownload)
    }

    /// Download the best available audio of a track into memory.
    ///
    /// Tries the signed lossless endpoint first (FLAC when the account and
    /// track support it), then falls back to the legacy MP3 variants.
    pub fn download_track(&self, track_id: &str) -> Result<DownloadedTrack> {
        if let Ok(downloaded) = self.download_lossless(track_id) {
            return Ok(downloaded);
        }

        let infos = self.download_info(track_id)?;
        let info = select_best_mp3(&infos).ok_or(Error::NoDownload)?.clone();
        let token = self.require_token()?;
        let xml = self
            .inner
            .http
            .get_text(&info.download_info_url, Some(&token))?;
        let direct_url = build_direct_link(&xml)?;
        let bytes =
            self.inner
                .http
                .get_bytes(&direct_url, Some(&token), Duration::from_secs(120))?;

        Ok(DownloadedTrack {
            bytes,
            direct_url,
            codec: info.codec,
            bitrate_kbps: info.bitrate_in_kbps,
        })
    }

    fn download_lossless(&self, track_id: &str) -> Result<DownloadedTrack> {
        let info = self.file_info(track_id)?;
        let file = info.playable().ok_or(Error::NoDownload)?;
        let direct_url = file.direct_url().ok_or(Error::NoDownload)?.to_string();
        let token = self.require_token()?;
        let bytes =
            self.inner
                .http
                .get_bytes(&direct_url, Some(&token), Duration::from_secs(180))?;

        Ok(DownloadedTrack {
            bytes,
            direct_url,
            codec: file.codec.clone(),
            bitrate_kbps: file.bitrate,
        })
    }

    /// Fetch the plain text of a track's lyrics.
    ///
    /// Returns `Ok(None)` when the track has no lyrics. The endpoint is signed
    /// like the Python client and the actual text is downloaded from the
    /// pre-signed URL in the response.
    pub fn track_lyrics(&self, track_id: &str) -> Result<Option<String>> {
        let token = self.require_token()?;
        let raw_id = track_id.split(':').next().unwrap_or(track_id);
        let (timestamp, sign) = lyrics_sign(raw_id)?;
        let url = self.api_url(&format!("/tracks/{raw_id}/lyrics"));
        let query = [
            ("format", "TEXT".to_string()),
            ("timeStamp", timestamp.to_string()),
            ("sign", sign),
        ];

        let lyrics: Option<TrackLyrics> = match self.inner.http.get(&url, &query, Some(&token)) {
            Ok(lyrics) => lyrics,
            Err(Error::NotFound(_)) => return Ok(None),
            Err(error) => return Err(error),
        };
        let Some(download_url) = lyrics.and_then(|lyrics| lyrics.download_url) else {
            return Ok(None);
        };
        let text = self.inner.http.get_text(&download_url, None)?;
        if text.trim().is_empty() {
            Ok(None)
        } else {
            Ok(Some(text))
        }
    }

    // ---------------------------------------------------------------- Covers

    /// Download the cover of a track at the requested size.
    pub fn download_cover(&self, track: &Track, size: u32) -> Result<Option<Vec<u8>>> {
        match track.cover_url(size) {
            Some(url) => self.download_image(&url).map(Some),
            None => Ok(None),
        }
    }

    /// Download an arbitrary image URL (covers are served without auth).
    pub fn download_image(&self, url: &str) -> Result<Vec<u8>> {
        self.inner
            .http
            .get_bytes(url, None, Duration::from_secs(30))
    }
}

/// Generate a random OAuth device id (10 alphanumeric characters).
fn random_device_id() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let mut hasher = DefaultHasher::new();
    now.as_nanos().hash(&mut hasher);
    std::process::id().hash(&mut hasher);
    std::thread::current().id().hash(&mut hasher);
    let hash = hasher.finish();

    let alphabet = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut state = hash;
    let mut id = String::with_capacity(10);
    for _ in 0..10 {
        id.push(alphabet[(state % alphabet.len() as u64) as usize] as char);
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
    }
    id
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_ids_look_valid() {
        for _ in 0..100 {
            let id = random_device_id();
            assert_eq!(id.len(), 10);
            assert!(id.chars().all(|ch| ch.is_ascii_alphanumeric()));
        }
    }

    #[test]
    fn client_builder_strips_trailing_slashes() {
        let client = ClientBuilder::new()
            .base_url("http://localhost:1234/")
            .oauth_base_url("http://localhost:1234/")
            .build();
        assert_eq!(client.inner.base_url, "http://localhost:1234");
        assert_eq!(client.inner.oauth_base_url, "http://localhost:1234");
    }

    #[test]
    fn setting_a_token_clears_uid() {
        let client = Client::unauthenticated();
        *client.inner.uid.write().unwrap() = Some(42);
        client.set_token("token");
        assert_eq!(client.cached_uid(), None);
        assert_eq!(client.token().as_deref(), Some("token"));
    }
}
