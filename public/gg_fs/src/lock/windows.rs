use std::fs::{File, OpenOptions};
use std::io;
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;

use windows_sys::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ, FILE_SHARE_WRITE,
};

use crate::sys;

/// Opens the lock file, creating it if needed. Fails if `path` is a reparse
/// point.
pub(super) fn open(path: &Path) -> io::Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        // Without FILE_SHARE_DELETE, the file cannot be deleted while it is open.
        // Python's `filelock` deletes the file after release, which would let a
        // third process lock a new file at the same path.
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
        .open(path)?;
    if file.metadata()?.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        Err(io::Error::new(io::ErrorKind::InvalidInput, "is a symbolic link"))
    } else {
        Ok(file)
    }
}

/// Tries to take the lock without waiting. Returns `false` if it is held.
pub(super) fn try_lock(file: &File) -> io::Result<bool> {
    sys::try_lock(file, 0)
}

/// Returns `false`.
pub(super) fn is_unlinked(_file: &File) -> io::Result<bool> {
    Ok(false)
}
