use std::path::Path;
use std::{fs, io};

use super::OP;
use crate::error::{Context, Result};
use crate::sys::ProtectedAccess;

pub(super) fn create_dir(path: &Path) -> Result<()> {
    ProtectedAccess::caller_only().and_then(|access| access.create_dir(path)).context(OP, path)
}

pub(super) fn create_file(path: &Path) -> io::Result<fs::File> {
    ProtectedAccess::caller_only()?.create_file(path)
}
