use std::fs::{self, OpenOptions};
use std::os::windows::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::{env, io};

use super::OP;
use crate::dir::create_dir_all;
use crate::error::{Error, Result};
use crate::metadata::{NO_FOLLOW, metadata_with};
use crate::sys::SHARE_ALL;

/// Returns the `gg_fs` subdirectory of the system temporary directory, creating
/// it if needed.
pub(super) fn root() -> Result<PathBuf> {
    let root = env::temp_dir().join("gg_fs");
    create_dir_all(&root)?;
    if metadata_with(&root, &NO_FOLLOW)?.is_symlink() {
        Err(Error::with_kind(OP, root, io::ErrorKind::InvalidInput, "is a symbolic link"))
    } else {
        Ok(root)
    }
}

pub(super) fn create_file(path: &Path) -> io::Result<fs::File> {
    OpenOptions::new().read(true).write(true).create_new(true).share_mode(SHARE_ALL).open(path)
}
