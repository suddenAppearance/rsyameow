//! Persistence of the OAuth token between launches.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const CONFIG_DIR: &str = "rsyameow";
const SESSION_FILE: &str = "session.json";
const WINDOW_FILE: &str = "window.json";
const PLAYBACK_FILE: &str = "playback.json";
const STATE_FILE: &str = "state.json";

#[derive(Debug, Serialize, Deserialize)]
struct Session {
    token: String,
}

/// Last observed window size, stored between launches.
#[derive(Debug, Serialize, Deserialize)]
struct WindowSize {
    width: f32,
    height: f32,
}

/// Playback preferences (currently only volume).
#[derive(Debug, Serialize, Deserialize)]
struct PlaybackSettings {
    volume: f32,
}

/// UI and playback state restored on the next launch.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppState {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub demo: bool,
    #[serde(default)]
    pub uid: Option<u64>,
    /// Stable screen key: profile, liked, search, playlists, playlist, wave,
    /// artist, album or track.
    #[serde(default)]
    pub screen: String,
    #[serde(default)]
    pub search_query: String,
    #[serde(default)]
    pub search_kind: String,
    #[serde(default)]
    pub queue: Vec<String>,
    #[serde(default)]
    pub queue_index: Option<usize>,
    #[serde(default)]
    pub track_id: Option<String>,
    #[serde(default)]
    pub page_track_id: Option<String>,
    #[serde(default)]
    pub position_ms: u64,
    #[serde(default)]
    pub quality: Option<String>,
    #[serde(default)]
    pub queue_open: bool,
    #[serde(default)]
    pub queue_source: String,
    #[serde(default)]
    pub playlist_id: Option<String>,
    #[serde(default)]
    pub artist_id: Option<u64>,
    #[serde(default)]
    pub album_id: Option<u64>,
}

/// Returns the directory used for the session file.
///
/// `RSYAMEOW_CONFIG_DIR` overrides the location, which is convenient for tests
/// and portable setups.
pub fn config_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("RSYAMEOW_CONFIG_DIR") {
        return Some(PathBuf::from(dir));
    }
    if let Some(dir) = std::env::var_os("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(dir).join(CONFIG_DIR));
    }
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config").join(CONFIG_DIR))
}

/// Load a previously saved token. Invalid or unreadable files are ignored.
pub fn load_token() -> Option<String> {
    let path = config_dir()?.join(SESSION_FILE);
    let data = fs::read_to_string(&path).ok()?;
    let session: Session = serde_json::from_str(&data).ok()?;
    let token = session.token.trim().to_string();
    if !token.is_empty() {
        harden_session_permissions(&path);
        Some(token)
    } else {
        None
    }
}

/// Save the token so that the next launch can log in automatically.
pub fn save_token(token: &str) -> io::Result<()> {
    let dir = ensure_config_dir()?;
    let path = dir.join(SESSION_FILE);
    let data = serde_json::to_vec_pretty(&Session {
        token: token.to_string(),
    })
    .map_err(io::Error::other)?;

    write_private(&path, &data)?;
    harden_session_permissions(&path);
    Ok(())
}

/// Load the window size remembered during the previous run.
pub fn load_window_size() -> Option<(f32, f32)> {
    let path = config_dir()?.join(WINDOW_FILE);
    let data = fs::read_to_string(path).ok()?;
    let size: WindowSize = serde_json::from_str(&data).ok()?;
    if size.width.is_finite()
        && size.height.is_finite()
        && size.width >= 320.0
        && size.height >= 240.0
    {
        Some((size.width, size.height))
    } else {
        None
    }
}

/// Persist the last window size for the next launch.
pub fn save_window_size(width: f32, height: f32) -> io::Result<()> {
    if !width.is_finite() || !height.is_finite() || width < 320.0 || height < 240.0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid window size",
        ));
    }

    let dir = ensure_config_dir()?;
    let data = serde_json::to_vec_pretty(&WindowSize {
        width: width.round(),
        height: height.round(),
    })
    .map_err(io::Error::other)?;
    fs::write(dir.join(WINDOW_FILE), data)
}

/// Load the saved player volume, clamped to `0.0..=1.0`.
pub fn load_volume() -> Option<f32> {
    let path = config_dir()?.join(PLAYBACK_FILE);
    let data = fs::read_to_string(path).ok()?;
    let settings: PlaybackSettings = serde_json::from_str(&data).ok()?;
    if settings.volume.is_finite() {
        Some(settings.volume.clamp(0.0, 1.0))
    } else {
        None
    }
}

/// Persist the player volume.
pub fn save_volume(volume: f32) -> io::Result<()> {
    if !volume.is_finite() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid volume",
        ));
    }
    let dir = ensure_config_dir()?;
    let data = serde_json::to_vec_pretty(&PlaybackSettings {
        volume: volume.clamp(0.0, 1.0),
    })
    .map_err(io::Error::other)?;
    fs::write(dir.join(PLAYBACK_FILE), data)
}

/// Load the last saved UI/playback state. Missing or malformed files are ignored.
pub fn load_app_state() -> Option<AppState> {
    let path = config_dir()?.join(STATE_FILE);
    let data = fs::read_to_string(path).ok()?;
    let state: AppState = serde_json::from_str(&data).ok()?;
    if state.version == 1 {
        Some(state)
    } else {
        None
    }
}

/// Persist UI and playback state for the next launch.
pub fn save_app_state(state: &AppState) -> io::Result<()> {
    let dir = ensure_config_dir()?;
    let data = serde_json::to_vec_pretty(state).map_err(io::Error::other)?;
    write_private(&dir.join(STATE_FILE), &data)
}

/// Remove the saved UI/playback state.
pub fn clear_app_state() {
    if let Some(path) = config_dir().map(|dir| dir.join(STATE_FILE)) {
        let _ = fs::remove_file(path);
    }
}

fn ensure_config_dir() -> io::Result<PathBuf> {
    let dir = config_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "config directory is unknown"))?;
    fs::create_dir_all(&dir)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&dir, fs::Permissions::from_mode(0o700));
    }

    Ok(dir)
}

fn harden_session_permissions(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
}

/// Remove the saved token.
pub fn clear_token() {
    if let Some(path) = config_dir().map(|dir| dir.join(SESSION_FILE)) {
        let _ = fs::remove_file(path);
    }
}

fn write_private(path: &Path, data: &[u8]) -> io::Result<()> {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        file.write_all(data)?;
        file.flush()?;
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
    }

    #[cfg(not(unix))]
    {
        fs::write(path, data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_roundtrip_uses_env_config_dir() {
        let dir = std::env::temp_dir().join(format!(
            "rsyameow-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        // SAFETY: tests are single-threaded for this variable.
        unsafe {
            std::env::set_var("RSYAMEOW_CONFIG_DIR", &dir);
            std::env::remove_var("XDG_CONFIG_HOME");
        }

        save_token("test-token").unwrap();
        assert_eq!(load_token().as_deref(), Some("test-token"));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(dir.join(SESSION_FILE))
                .unwrap()
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, 0o600);
        }

        save_window_size(1200.0, 800.0).unwrap();
        assert_eq!(load_window_size(), Some((1200.0, 800.0)));

        save_volume(0.42).unwrap();
        assert_eq!(load_volume(), Some(0.42));

        let state = AppState {
            version: 1,
            demo: true,
            uid: Some(42),
            screen: "liked".into(),
            search_query: "кот".into(),
            search_kind: "artists".into(),
            queue: vec!["1".into(), "2:3".into()],
            queue_index: Some(1),
            track_id: Some("2".into()),
            position_ms: 12_345,
            quality: Some("FLAC · Lossless".into()),
            queue_open: true,
            ..Default::default()
        };
        save_app_state(&state).unwrap();
        let loaded = load_app_state().expect("state");
        assert_eq!(loaded.screen, "liked");
        assert_eq!(loaded.queue, vec!["1", "2:3"]);
        assert_eq!(loaded.position_ms, 12_345);

        clear_app_state();
        assert!(load_app_state().is_none());

        clear_token();
        assert_eq!(load_token(), None);
        assert_eq!(load_window_size(), Some((1200.0, 800.0)));

        let _ = fs::remove_dir_all(dir);
        unsafe {
            std::env::remove_var("RSYAMEOW_CONFIG_DIR");
        }
    }
}
