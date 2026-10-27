use std::fs::{File, OpenOptions};
use std::io;
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
use std::os::windows::io::AsRawHandle;
use std::path::Path;

use windows_sys::Win32::Foundation::ERROR_LOCK_VIOLATION;
use windows_sys::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ, FILE_SHARE_WRITE,
    LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY, LockFileEx,
};
use windows_sys::Win32::System::IO::OVERLAPPED;

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
    // SAFETY: an all-zero `OVERLAPPED` is the documented initial state.
    let mut overlapped: OVERLAPPED = unsafe { std::mem::zeroed() };
    // SAFETY: `file` owns a live handle and `overlapped` outlives the call.
    let locked = unsafe {
        LockFileEx(
            file.as_raw_handle() as _,
            LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY,
            0,
            1,
            0,
            &mut overlapped,
        )
    };
    if locked != 0 {
        Ok(true)
    } else {
        let err = io::Error::last_os_error();
        if err.raw_os_error() == Some(ERROR_LOCK_VIOLATION as i32) { Ok(false) } else { Err(err) }
    }
}

/// Returns `false`.
pub(super) fn is_unlinked(_file: &File) -> io::Result<bool> {
    Ok(false)
}
