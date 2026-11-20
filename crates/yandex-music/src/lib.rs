//! Unofficial client for the [Yandex Music](https://music.yandex.ru) API.
//!
//! The crate is a Rust port of a useful subset of the Python
//! [`yandex-music-api`](https://github.com/MarshalX/yandex-music-api) library. It
//! intentionally keeps the same endpoints, request shapes and authentication
//! quirks, but exposes an idiomatic, synchronous Rust API.
//!
//! # Getting a token
//!
//! Yandex Music accepts OAuth tokens in the `Authorization: OAuth <token>`
//! header. The easiest way to obtain one is the OAuth device flow implemented by
//! [`Client::request_device_code`] and [`Client::poll_device_token`]:
//!
//! ```no_run
//! use std::time::Duration;
//! use yandex_music::Client;
//!
//! # fn main() -> yandex_music::Result<()> {
//! let client = Client::unauthenticated();
//! let code = client.request_device_code()?;
//! println!("Open {} and enter {}", code.verification_url, code.user_code);
//!
//! loop {
//!     if let Some(token) = client.poll_device_token(&code.device_code)? {
//!         client.set_token(token.access_token);
//!         break;
//!     }
//!     std::thread::sleep(Duration::from_secs(code.interval));
//! }
//!
//! let status = client.init()?;
//! println!("Hello, {:?}", status.account.and_then(|a| a.display_name));
//! # Ok(())
//! # }
//! ```
//!
//! # A random liked track
//!
//! ```no_run
//! use yandex_music::Client;
//!
//! # fn main() -> yandex_music::Result<()> {
//! let client = Client::with_token(std::env::var("YANDEX_MUSIC_TOKEN").unwrap());
//! client.init()?;
//!
//! let liked = client.liked_tracks()?;
//! if let Some(short) = liked.tracks.first() {
//!     let track = client.track(&short.track_id())?;
//!     println!("{} — {}", track.artist_names(), track.title.as_deref().unwrap_or(""));
//!
//!     let downloaded = client.download_track(&track.id)?;
//!     std::fs::write("song.mp3", downloaded.bytes).expect("write track");
//! }
//! # Ok(())
//! # }
//! ```
//!
//! This is an unofficial project. It is not affiliated with or endorsed by
//! Yandex. Use it in accordance with Yandex's terms of service.

mod auth;
mod client;
mod download;
mod error;
mod http;
mod models;

pub use auth::{DEFAULT_CLIENT_ID, DEFAULT_CLIENT_SECRET, DeviceCode, OAuthToken};
pub use client::{Client, ClientBuilder};
pub use download::{
    DownloadInfo, DownloadedTrack, FILE_INFO_CLIENT, FILE_INFO_CODECS, FILE_INFO_SIGN_KEY,
    FileDownloadInfo, FileInfo, SIGN_SALT, build_direct_link, file_info_sign, file_info_sign_at,
};
pub use error::{Error, Result};
pub use models::{
    Account, Album, Artist, Cover, Permissions, Playlist, Plus, SearchResult, Status, Subscription,
    Track, TrackShort, TracksList, User,
};
