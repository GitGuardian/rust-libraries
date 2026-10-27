use std::ffi::CStr;
use std::fs::File;
use std::io;
use std::mem::MaybeUninit;
use std::os::unix::io::AsRawFd;
use std::path::Path;

use super::unix::c_path;
pub(crate) use super::unix::{
    file_id, geteuid, host_umask, passwd_home, path_id, set_blocking, set_umask,
};

/// Returns the name of the file system type at `path`.
pub(crate) fn fs_type_name(path: &Path) -> io::Result<String> {
    let path = c_path(path)?;
    let mut stat = MaybeUninit::<libc::statfs>::uninit();
    if unsafe { libc::statfs(path.as_ptr(), stat.as_mut_ptr()) } == -1 {
        Err(io::Error::last_os_error())
    } else {
        let stat = unsafe { stat.assume_init() };
        Ok(unsafe { CStr::from_ptr(stat.f_fstypename.as_ptr()) }.to_string_lossy().into_owned())
    }
}

/// Disables caching of reads on `file`, ignoring failure.
pub(crate) fn set_no_cache(file: &File) {
    unsafe { libc::fcntl(file.as_raw_fd(), libc::F_NOCACHE, 1) };
}

/// Returns whether the file system at `path` distinguishes names by case.
pub(crate) fn case_sensitive(path: &Path) -> io::Result<bool> {
    let path = c_path(path)?;
    match unsafe { libc::pathconf(path.as_ptr(), libc::_PC_CASE_SENSITIVE) } {
        -1 => Err(io::Error::last_os_error()),
        0 => Ok(false),
        _ => Ok(true),
    }
}

pub(crate) fn current_umask() -> u32 {
    super::unix::probe_umask()
}
