//! Persistence of the OAuth token between launches.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const CONFIG_DIR: &str = "rsyameow";
const SESSION_FILE: &str = "session.json";

#[derive(Debug, Serialize, Deserialize)]
struct Session {
    token: String,
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
    let data = fs::read_to_string(path).ok()?;
    let session: Session = serde_json::from_str(&data).ok()?;
    let token = session.token.trim().to_string();
    (!token.is_empty()).then_some(token)
}

/// Save the token so that the next launch can log in automatically.
pub fn save_token(token: &str) -> io::Result<()> {
    let dir = config_dir()
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "config directory is unknown"))?;
    fs::create_dir_all(&dir)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&dir, fs::Permissions::from_mode(0o700));
    }

    let path = dir.join(SESSION_FILE);
    let data = serde_json::to_vec_pretty(&Session {
        token: token.to_string(),
    })
    .map_err(io::Error::other)?;

    write_private(&path, &data)
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
        file.flush()
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

        clear_token();
        assert_eq!(load_token(), None);

        let _ = fs::remove_dir_all(dir);
        unsafe {
            std::env::remove_var("RSYAMEOW_CONFIG_DIR");
        }
    }
}
