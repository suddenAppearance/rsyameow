//! Error type shared by every client operation.

use thiserror::Error;

/// Result alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Everything that can go wrong while talking to Yandex Music.
#[derive(Debug, Error)]
pub enum Error {
    /// DNS, TCP, TLS or another transport-level failure.
    #[error("network error: {0}")]
    Network(String),

    /// The request exceeded its configured timeout.
    #[error("request timed out")]
    Timeout,

    /// The token is missing or rejected by the server.
    #[error("unauthorized: {0}")]
    Unauthorized(String),

    /// The requested object does not exist.
    #[error("not found: {0}")]
    NotFound(String),

    /// The server rejected the request payload.
    #[error("bad request: {0}")]
    BadRequest(String),

    /// Any other non-successful HTTP response.
    #[error("API error ({status}): {message}")]
    Api { status: u16, message: String },

    /// The response could not be deserialized.
    #[error("invalid API response: {0}")]
    Decode(String),

    /// OAuth device authorization failed (expired code, access denied, ...).
    #[error("device authorization failed: {0}")]
    DeviceAuth(String),

    /// No playable MP3 variant was returned for the track.
    #[error("no suitable track download found")]
    NoDownload,

    /// The operation requires an OAuth token.
    #[error("the client is not authenticated")]
    NotAuthenticated,

    /// The account id is not known yet.
    #[error("account uid is unknown, call `init()` first")]
    UnknownAccount,
}
