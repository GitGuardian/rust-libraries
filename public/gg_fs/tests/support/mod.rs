#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

pub struct Scratch {
    _dir: TempDir,
    root: PathBuf,
}

impl Scratch {
    pub fn new() -> Self {
        let dir = TempDir::new().unwrap();
        let root = gg_fs::paths::canonicalize(dir.path()).unwrap();
        Self { _dir: dir, root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    pub fn write(&self, relative: &str, contents: impl AsRef<[u8]>) -> PathBuf {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, contents).unwrap();
        path
    }

    pub fn dir(&self, relative: &str) -> PathBuf {
        let path = self.path(relative);
        fs::create_dir_all(&path).unwrap();
        path
    }

    pub fn names(&self, relative: &str) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(self.path(relative))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
}

#[cfg(unix)]
pub fn hold_lock(path: &Path) -> fs::File {
    use std::os::fd::AsRawFd;
    let file = fs::File::open(path).unwrap();
    assert_eq!(unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) }, 0);
    file
}

#[cfg(windows)]
pub fn hold_lock(path: &Path) -> fs::File {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY, LockFileEx,
    };
    use windows_sys::Win32::System::IO::OVERLAPPED;
    let file = fs::File::open(path).unwrap();
    let mut overlapped: OVERLAPPED = unsafe { std::mem::zeroed() };
    let locked = unsafe {
        LockFileEx(
            file.as_raw_handle() as _,
            LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY,
            0,
            u32::MAX,
            u32::MAX,
            &mut overlapped,
        )
    };
    assert_ne!(locked, 0);
    file
}

#[cfg(unix)]
pub mod unix {
    use std::fs;
    use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
    use std::path::Path;

    pub fn set_mode(path: &Path, mode: u32) {
        fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
    }

    pub fn mode_of(path: &Path) -> u32 {
        fs::metadata(path).unwrap().permissions().mode() & 0o7777
    }

    pub fn inode_of(path: &Path) -> u64 {
        fs::metadata(path).unwrap().ino()
    }

    pub fn link_count(path: &Path) -> u64 {
        fs::metadata(path).unwrap().nlink()
    }

    pub fn link(target: &Path, link: &Path) {
        symlink(target, link).unwrap();
    }

    pub fn make_fifo(path: &Path) {
        let c_path = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(c_path.as_ptr(), 0o600) }, 0);
    }

    pub fn euid() -> u32 {
        unsafe { libc::geteuid() }
    }

    pub fn is_root() -> bool {
        euid() == 0
    }

    pub fn owner_of(path: &Path) -> (u32, u32) {
        let metadata = fs::symlink_metadata(path).unwrap();
        (metadata.uid(), metadata.gid())
    }

    pub fn chown(path: &Path, uid: u32, gid: u32) {
        std::os::unix::fs::chown(path, Some(uid), Some(gid)).unwrap();
    }

    pub fn other_group(excluded: u32) -> Option<u32> {
        let count = unsafe { libc::getgroups(0, std::ptr::null_mut()) };
        let mut groups = vec![0 as libc::gid_t; usize::try_from(count).unwrap()];
        let count = unsafe { libc::getgroups(count, groups.as_mut_ptr()) };
        groups.truncate(usize::try_from(count).unwrap());
        groups.into_iter().find(|gid| *gid != excluded)
    }
}
