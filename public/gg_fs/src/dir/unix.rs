use std::fs::{DirBuilder, Permissions};
use std::io;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::Path;

use super::OP;
use crate::error::{Context, Result};
use crate::file::{File, OpenOptions};
use crate::mode::Mode;

const ALL_BITS: u32 = 0o777;
const OWNER_ONLY: u32 = 0o700;

pub(super) fn create_dir(path: &Path, mode: Mode) -> io::Result<()> {
    let requested = match mode {
        Mode::Exact(_) => OWNER_ONLY,
        Mode::Umask | Mode::HostUmask => ALL_BITS,
    };
    DirBuilder::new().mode(requested).create(path)
}

/// Sets the mode of a directory, without following symbolic links.
pub(crate) fn set_mode(path: &Path, mode: Mode) -> Result<()> {
    mode.explicit_bits(ALL_BITS).map_or(Ok(()), |bits| {
        let options = OpenOptions { follow_symlinks: false, ..OpenOptions::default() };
        let dir = File::open_with(path, &options)?;
        dir.set_permissions(Permissions::from_mode(bits)).context(OP, path)
    })
}
