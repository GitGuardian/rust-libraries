use std::fs::{File, OpenOptions};
use std::io;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;

pub(super) use crate::sys::try_lock;

/// Opens the lock file, creating it with mode `0o600`. Fails if `path` is a
/// symbolic link.
pub(super) fn open(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
}

/// Returns whether `file` has no links left.
pub(super) fn is_unlinked(file: &File) -> io::Result<bool> {
    Ok(file.metadata()?.nlink() == 0)
}
