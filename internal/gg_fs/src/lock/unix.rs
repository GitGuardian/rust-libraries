use std::fs::{File, OpenOptions};
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;

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

/// Tries to take the lock without waiting. Returns `false` if it is held.
pub(super) fn try_lock(file: &File) -> io::Result<bool> {
    // SAFETY: `file` owns a live descriptor for the whole call, and `flock`
    // only inspects it.
    let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if result == 0 {
        Ok(true)
    } else {
        let err = io::Error::last_os_error();
        if err.kind() == io::ErrorKind::WouldBlock { Ok(false) } else { Err(err) }
    }
}

/// Returns whether `file` has no links left.
pub(super) fn is_unlinked(file: &File) -> io::Result<bool> {
    Ok(file.metadata()?.nlink() == 0)
}
