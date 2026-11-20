//! OAuth device flow models and defaults.
//!
//! Yandex's OAuth server supports the device authorization grant. The client
//! id/secret below are the public credentials of the official Android Yandex
//! Music application as documented by the `yandex-music-api` Python project.

use serde::Deserialize;

/// Public OAuth client id of the Android Yandex Music application.
pub const DEFAULT_CLIENT_ID: &str = "23cabbbdc6cd418abb4b39c32c41195d";
/// Public OAuth client secret of the Android Yandex Music application.
pub const DEFAULT_CLIENT_SECRET: &str = "53bc75238f0c4d08a118e51fe9203300";
/// Default device name shown on the Yandex confirmation page.
pub const DEFAULT_DEVICE_NAME: &str = "rsyameow";

/// A device code returned by `POST https://oauth.yandex.ru/device/code`.
#[derive(Debug, Clone, Deserialize)]
pub struct DeviceCode {
    /// Opaque code used when polling for the token.
    pub device_code: String,
    /// Short code the user enters on the confirmation page.
    pub user_code: String,
    /// URL of the confirmation page.
    pub verification_url: String,
    /// Number of seconds until the device code expires.
    #[serde(default)]
    pub expires_in: u64,
    /// Server-recommended polling interval in seconds.
    #[serde(default = "default_interval")]
    pub interval: u64,
}

fn default_interval() -> u64 {
    5
}

/// An OAuth access token returned by `POST https://oauth.yandex.ru/token`.
#[derive(Debug, Clone, Deserialize)]
pub struct OAuthToken {
    /// The token itself.
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub expires_in: Option<u64>,
    #[serde(default)]
    pub token_type: Option<String>,
}
