use std::fs::DirBuilder;
use std::io;
use std::path::Path;

use crate::error::Result;
use crate::mode::Mode;
use crate::sys::ProtectedAccess;

/// Creates one directory.
pub(super) fn create_dir(path: &Path, mode: Mode) -> io::Result<()> {
    if mode == Mode::WorldReadable {
        ProtectedAccess::world_readable()?.create_dir(path)
    } else {
        DirBuilder::new().create(path)
    }
}

/// Does nothing.
pub(super) fn set_mode(_path: &Path, _mode: Mode) -> Result<()> {
    Ok(())
}
