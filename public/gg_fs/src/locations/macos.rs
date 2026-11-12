use std::fs;
use std::path::{Path, PathBuf};

use super::{Env, unix};

pub(super) use super::unix::{env_key, os_home};

const USERS: &str = "/Users";

/// Returns the application directory in `~/Library/Application Support`.
pub(super) fn config_dir(_env: &Env, home: &Path, app: &str) -> PathBuf {
    home.join("Library/Application Support").join(app)
}

/// Returns the application directory in `~/Library/Caches`.
pub(super) fn cache_dir(_env: &Env, home: &Path, app: &str) -> PathBuf {
    home.join("Library/Caches").join(app)
}

/// Returns `/var/db/<application>` if the process runs as root and `home`
/// belongs to another user.
pub(super) fn system_state_dir(home: &Path, app: &str) -> Option<PathBuf> {
    unix::system_state_dir(home, Path::new("/var/db"), app)
}

/// Returns the user directories in `/Users`, and `/var/root`.
pub(super) fn homes() -> Vec<PathBuf> {
    let mut homes = Vec::new();
    if let Ok(entries) = fs::read_dir(USERS) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let is_user = !name.to_string_lossy().starts_with('.') && name != "Shared";
            if is_user && entry.path().is_dir() {
                homes.push(entry.path());
            }
        }
    }
    homes.push(PathBuf::from("/var/root"));
    homes
}
