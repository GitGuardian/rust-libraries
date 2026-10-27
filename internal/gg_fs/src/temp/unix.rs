use std::fs::{self, OpenOptions};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::{env, io};

use super::OP;
use crate::dir::{create_dir_all, set_mode};
use crate::error::{Cause, Error, Result, UntrustedReason};
use crate::metadata::{NO_FOLLOW, metadata_with};
use crate::mode::Mode;
use crate::sys;

const OWNER_ONLY: u32 = 0o700;
const GROUP_AND_OTHER: u32 = 0o077;

/// Returns `gg_fs-<uid>` in the system temporary directory, creating it with
/// mode `0o700`.
pub(super) fn root() -> Result<PathBuf> {
    let uid = sys::geteuid();
    let root = env::temp_dir().join(format!("gg_fs-{uid}"));
    create_dir_all(&root)?;
    let metadata = metadata_with(&root, &NO_FOLLOW)?;
    if metadata.uid() != uid {
        Err(Error::new(OP, root, Cause::Untrusted(UntrustedReason::Owner)))
    } else if metadata.mode() & GROUP_AND_OTHER != 0 {
        set_mode(&root, Mode::Exact(OWNER_ONLY))?;
        Ok(root)
    } else {
        Ok(root)
    }
}

pub(super) fn create_file(path: &Path) -> io::Result<fs::File> {
    OpenOptions::new().read(true).write(true).create_new(true).mode(0o600).open(path)
}
