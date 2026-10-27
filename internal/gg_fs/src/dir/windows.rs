use std::fs::DirBuilder;
use std::io;
use std::path::Path;

use crate::error::Result;
use crate::mode::Mode;

/// Creates one directory. `mode` is ignored.
pub(super) fn create_dir(path: &Path, _mode: Mode) -> io::Result<()> {
    DirBuilder::new().create(path)
}

/// Does nothing.
pub(crate) fn set_mode(_path: &Path, _mode: Mode) -> Result<()> {
    Ok(())
}
