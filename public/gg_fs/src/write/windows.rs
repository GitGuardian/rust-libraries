use std::fs::{self, Metadata, OpenOptions};
use std::io;
use std::os::windows::fs::OpenOptionsExt;
use std::path::Path;

use crate::error::Result;
use crate::mode::Mode;
use crate::sys::{ProtectedAccess, SHARE_ALL};

/// Creates `path` exclusively for writing.
pub(super) fn create_temp(path: &Path, mode: Mode) -> io::Result<fs::File> {
    if mode == Mode::WorldReadable {
        ProtectedAccess::world_readable()?.create_file(path)
    } else {
        OpenOptions::new().write(true).create_new(true).share_mode(SHARE_ALL).open(path)
    }
}

/// Does nothing.
pub(super) fn set_new_mode(_file: &fs::File, _mode: Mode) -> io::Result<()> {
    Ok(())
}

/// Does nothing.
pub(super) fn set_exact_mode(_file: &fs::File, _mode: Mode) -> io::Result<()> {
    Ok(())
}

/// Returns `true`.
pub(super) fn has_mode(_metadata: &Metadata, _mode: Mode) -> bool {
    true
}

/// Does nothing.
pub(super) fn sync_dir(_dir: &Path) -> Result<()> {
    Ok(())
}
