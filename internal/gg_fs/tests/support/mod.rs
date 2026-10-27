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
}
