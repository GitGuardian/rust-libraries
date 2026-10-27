use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use super::Env;
use crate::sys;

const APP_AUTHOR: &str = "GitGuardian";
const NON_USER_DIRS: [&str; 6] =
    ["Public", "Default", "Default User", "All Users", "WDAGUtilityAccount", "defaultuser0"];

pub(super) fn env_key(name: OsString) -> OsString {
    name.to_ascii_uppercase()
}

/// Returns the profile directory of the current user.
pub(super) fn os_home(env: &Env) -> Option<PathBuf> {
    env.absolute_path("USERPROFILE")
        .or_else(|| sys::profile_dir().filter(|home| home.is_absolute()))
}

/// Returns `None`.
pub(super) fn system_state_dir(_home: &Path, _app: &str) -> Option<PathBuf> {
    None
}

/// Returns the application directory in `GitGuardian` in `%LOCALAPPDATA%`.
pub(super) fn config_dir(env: &Env, home: &Path, app: &str) -> PathBuf {
    let local =
        env.absolute_path("LOCALAPPDATA").unwrap_or_else(|| home.join("AppData").join("Local"));
    local.join(APP_AUTHOR).join(app)
}

/// Returns the `Cache` directory in the default configuration directory.
pub(super) fn cache_dir(env: &Env, home: &Path, app: &str) -> PathBuf {
    config_dir(env, home, app).join("Cache")
}

/// Returns the readable directories in the `Users` directory of every drive.
pub(super) fn homes() -> Vec<PathBuf> {
    let mut homes = Vec::new();
    for letter in 'A'..='Z' {
        if let Ok(entries) = fs::read_dir(format!("{letter}:\\Users")) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let is_user =
                    !NON_USER_DIRS.iter().any(|skipped| name.eq_ignore_ascii_case(skipped));
                if is_user && entry.path().is_dir() {
                    homes.push(entry.path());
                }
            }
        }
    }
    homes
}
