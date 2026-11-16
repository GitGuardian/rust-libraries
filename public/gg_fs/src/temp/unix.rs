use std::fs::{self, DirBuilder, OpenOptions};
use std::io;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::Path;

use super::OP;
use crate::error::{Context, Result};

pub(super) fn create_dir(path: &Path) -> Result<()> {
    DirBuilder::new().mode(0o700).create(path).context(OP, path)
}

pub(super) fn create_file(path: &Path) -> io::Result<fs::File> {
    OpenOptions::new().read(true).write(true).create_new(true).mode(0o600).open(path)
}
