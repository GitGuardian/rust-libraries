use std::fs::File;
use std::path::Path;

use crate::error::Result;

#[cfg_attr(unix, path = "unix.rs")]
#[cfg_attr(windows, path = "windows.rs")]
mod imp;

/// Requirements a file must meet to be read.
///
/// Except for [`Trust::Unchecked`], a trusted file is a regular file, not a
/// symbolic link, owned by an accepted user, and not writable by group or
/// others.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Trust {
    /// No requirements.
    #[default]
    Unchecked,
    /// Owned by the calling user.
    Caller,
    /// Owned by the calling user, root, or the owner of the home directory.
    RootOrHomeOwner,
    /// Owned by root (uid 0).
    RootOnly,
}

/// Checks an open file against `trust`.
///
/// # Platform-specific behavior
///
/// On Windows ownership and access rights are checked in place of the owner and
/// mode. The local system account and the Administrators group count as root.
/// The default owner of the calling token is accepted as the calling user. A
/// file whose ownership cannot be read fails with an I/O error.
pub(crate) fn verify(file: &File, path: &Path, trust: &Trust) -> Result<()> {
    imp::verify(file, path, trust)
}
