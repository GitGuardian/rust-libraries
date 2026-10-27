use std::fs::{self, File};
use std::io;
use std::mem::MaybeUninit;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use super::containing_dir;
use super::unix::{c_path, home_of, probe_umask};

pub(crate) use super::unix::{
    file_id, geteuid, host_umask, passwd_home, path_id, set_blocking, set_umask,
};

const CASEFOLD_FLAG: libc::c_int = 0x4000_0000;
const CASE_INSENSITIVE_MAGICS: [u64; 8] =
    [0x4d44, 0x5346_544e, 0x2011_bab0, 0x517b, 0xff53_4d42, 0x7366_746e, 0x4244, 0x482b];

static PASSWD_LOCK: Mutex<()> = Mutex::new(());

/// Returns whether the directory of `path` distinguishes names by case.
pub(crate) fn case_sensitive(path: &Path) -> io::Result<bool> {
    let dir = File::open(containing_dir(path)?)?;
    let fd = dir.as_raw_fd();
    let mut stat = MaybeUninit::<libc::statfs>::uninit();
    if unsafe { libc::fstatfs(fd, stat.as_mut_ptr()) } == -1 {
        Err(io::Error::last_os_error())
    } else {
        let stat = unsafe { stat.assume_init() };
        if CASE_INSENSITIVE_MAGICS.contains(&magic_of(&stat)) {
            Ok(false)
        } else {
            let mut flags: libc::c_int = 0;
            let status = unsafe { libc::ioctl(fd, libc::FS_IOC_GETFLAGS, &mut flags) };
            Ok(status == -1 || flags & CASEFOLD_FLAG == 0)
        }
    }
}

/// Returns the type of the file system at `path`.
pub(crate) fn fs_magic(path: &Path) -> io::Result<u64> {
    let path = c_path(path)?;
    let mut stat = MaybeUninit::<libc::statfs>::uninit();
    if unsafe { libc::statfs(path.as_ptr(), stat.as_mut_ptr()) } == -1 {
        Err(io::Error::last_os_error())
    } else {
        let stat = unsafe { stat.assume_init() };
        Ok(magic_of(&stat))
    }
}

#[allow(clippy::unnecessary_cast)]
fn magic_of(stat: &libc::statfs) -> u64 {
    stat.f_type as u64 & 0xffff_ffff
}

/// Returns the user id and home directory of every account.
pub(crate) fn passwd_homes() -> Vec<(u32, PathBuf)> {
    let _guard = PASSWD_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
    let mut homes = Vec::new();
    unsafe { libc::setpwent() };
    while let Some(entry) = unsafe { libc::getpwent().as_ref() } {
        homes.extend(home_of(entry).map(|home| (entry.pw_uid, home)));
    }
    unsafe { libc::endpwent() };
    homes
}

pub(crate) fn current_umask() -> u32 {
    proc_umask().unwrap_or_else(probe_umask)
}

fn proc_umask() -> Option<u32> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    let mask = status.lines().find_map(|line| line.strip_prefix("Umask:"))?;
    u32::from_str_radix(mask.trim(), 8).ok()
}
