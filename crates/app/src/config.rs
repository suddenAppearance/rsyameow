//! Persistence of the OAuth token between launches.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const CONFIG_DIR: &str = "rsyameow";
const SESSION_FILE: &str = "session.json";
const WINDOW_FILE: &str = "window.json";

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

        clear_token();
        assert_eq!(load_token(), None);
        assert_eq!(load_window_size(), Some((1200.0, 800.0)));

        let _ = fs::remove_dir_all(dir);
        unsafe {
            std::env::remove_var("RSYAMEOW_CONFIG_DIR");
        }
    }
}
