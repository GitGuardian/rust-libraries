use std::ffi::OsString;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use super::Env;
use crate::metadata::metadata;
use crate::sys;

pub(super) fn env_key(name: OsString) -> OsString {
    name
}

/// Returns the home directory of the effective user.
pub(super) fn os_home(env: &Env) -> Option<PathBuf> {
    let passwd = sys::passwd_home(sys::geteuid()).filter(|home| home.is_absolute());
    passwd.or_else(|| env.absolute_path("HOME"))
}

/// Returns the system state directory of `app` in `base` for a root process
/// whose `home` belongs to another user.
pub(super) fn system_state_dir(home: &Path, base: &Path, app: &str) -> Option<PathBuf> {
    let home_owner = metadata(home).ok()?.uid();
    (sys::geteuid() == 0 && home_owner != 0).then(|| base.join(app))
}
