use std::ffi::{CStr, CString, OsStr};
use std::fs::{self, File};
use std::mem::MaybeUninit;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};
use std::{io, ptr};

use super::{FileId, current_umask};

const DEFAULT_PASSWD_BUFFER: usize = 1024;
const MAX_PASSWD_BUFFER: usize = 1 << 20;

static HOST_UMASK: OnceLock<u32> = OnceLock::new();
static UMASK_LOCK: Mutex<()> = Mutex::new(());

/// Returns the id of `file`.
pub(crate) fn file_id(file: &File) -> io::Result<FileId> {
    file.metadata().map(FileId::from)
}

/// Returns the id of the file at `path`, following symbolic links.
pub(crate) fn path_id(path: &Path) -> io::Result<FileId> {
    fs::metadata(path).map(FileId::from)
}

impl From<fs::Metadata> for FileId {
    fn from(metadata: fs::Metadata) -> Self {
        Self { volume: metadata.dev(), index: u128::from(metadata.ino()) }
    }
}

/// Puts `file` in blocking mode.
pub(crate) fn set_blocking(file: &File) -> io::Result<()> {
    let fd = file.as_raw_fd();
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    let updated = if flags == -1 {
        -1
    } else {
        unsafe { libc::fcntl(fd, libc::F_SETFL, flags & !libc::O_NONBLOCK) }
    };
    if updated == -1 { Err(io::Error::last_os_error()) } else { Ok(()) }
}

/// Returns the effective user id of the process.
pub(crate) fn geteuid() -> u32 {
    unsafe { libc::geteuid() }
}

/// Returns the home directory of the user `uid`.
pub(crate) fn passwd_home(uid: u32) -> Option<PathBuf> {
    let size = unsafe { libc::sysconf(libc::_SC_GETPW_R_SIZE_MAX) };
    let mut buffer =
        vec![
            0u8;
            usize::try_from(size).ok().filter(|size| *size > 0).unwrap_or(DEFAULT_PASSWD_BUFFER)
        ];
    loop {
        let mut passwd = MaybeUninit::<libc::passwd>::uninit();
        let mut result = ptr::null_mut();
        let status = unsafe {
            libc::getpwuid_r(
                uid,
                passwd.as_mut_ptr(),
                buffer.as_mut_ptr().cast(),
                buffer.len(),
                &mut result,
            )
        };
        if status == libc::EINTR {
            continue;
        } else if status == libc::ERANGE && buffer.len() < MAX_PASSWD_BUFFER {
            buffer.resize(buffer.len() * 2, 0);
        } else {
            return if status == 0 && !result.is_null() {
                home_of(unsafe { &*result })
            } else {
                None
            };
        }
    }
}

pub(super) fn c_path(path: &Path) -> io::Result<CString> {
    CString::new(path.as_os_str().as_bytes())
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))
}

pub(super) fn home_of(entry: &libc::passwd) -> Option<PathBuf> {
    if entry.pw_dir.is_null() {
        None
    } else {
        let dir = unsafe { CStr::from_ptr(entry.pw_dir) }.to_bytes();
        (!dir.is_empty()).then(|| PathBuf::from(OsStr::from_bytes(dir)))
    }
}

/// Returns the umask of the invoking shell, provided the process changes its
/// umask only through [`set_umask`].
pub(crate) fn host_umask() -> u32 {
    let _guard = lock_umask();
    *HOST_UMASK.get_or_init(current_umask)
}

/// Sets the umask of the process and returns the previous one. The first call
/// records the previous umask as the host umask.
pub(crate) fn set_umask(mask: u32) -> u32 {
    let _guard = lock_umask();
    let previous = swap_umask(mask);
    HOST_UMASK.get_or_init(|| previous);
    previous
}

fn lock_umask() -> MutexGuard<'static, ()> {
    UMASK_LOCK.lock().unwrap_or_else(PoisonError::into_inner)
}

pub(super) fn probe_umask() -> u32 {
    let current = swap_umask(0o777);
    swap_umask(current);
    current
}

#[allow(clippy::useless_conversion)]
fn swap_umask(mask: u32) -> u32 {
    let mask = libc::mode_t::try_from(mask & 0o777).unwrap_or_default();
    u32::from(unsafe { libc::umask(mask) })
}
