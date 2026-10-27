use std::path::{Path, PathBuf};

use super::{Env, unix};
use crate::sys;

pub(super) use super::unix::{env_key, os_home};

const FIRST_USER_UID: u32 = 1000;

/// Returns the application directory in `$XDG_CONFIG_HOME`, or `~/.config`.
pub(super) fn config_dir(env: &Env, home: &Path, app: &str) -> PathBuf {
    xdg_dir(env, "XDG_CONFIG_HOME", &home.join(".config"), app)
}

/// Returns the application directory in `$XDG_CACHE_HOME`, or `~/.cache`.
pub(super) fn cache_dir(env: &Env, home: &Path, app: &str) -> PathBuf {
    xdg_dir(env, "XDG_CACHE_HOME", &home.join(".cache"), app)
}

/// Returns `/var/lib/<application>` if the process runs as root and `home`
/// belongs to another user.
pub(super) fn system_state_dir(home: &Path, app: &str) -> Option<PathBuf> {
    unix::system_state_dir(home, Path::new("/var/lib"), app)
}

/// Returns the home directories of the regular users, and `/root`.
pub(super) fn homes() -> Vec<PathBuf> {
    let users = sys::passwd_homes().into_iter();
    let mut homes: Vec<PathBuf> =
        users.filter(|(uid, _)| *uid >= FIRST_USER_UID).map(|(_, home)| home).collect();
    homes.push(PathBuf::from("/root"));
    homes
}

fn xdg_dir(env: &Env, var: &str, default: &Path, app: &str) -> PathBuf {
    let base = env.absolute_path(var);
    base.unwrap_or_else(|| default.to_path_buf()).join(app)
}
