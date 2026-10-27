use std::fs::{self, Metadata, OpenOptions, Permissions};
use std::io;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::Path;

use super::OP;
use crate::error::{Context, Result};
use crate::file::File;
use crate::mode::Mode;

const FILE_BITS: u32 = 0o666;

/// Creates `path` exclusively for writing, never with a wider mode than the
/// final one.
pub(super) fn create_temp(path: &Path, mode: Mode) -> io::Result<fs::File> {
    let requested = mode.explicit_bits(FILE_BITS).unwrap_or(FILE_BITS);
    OpenOptions::new().write(true).create_new(true).mode(requested).open(path)
}

/// Sets the permission bits that `mode` gives a new file, if it gives any.
pub(super) fn set_new_mode(file: &fs::File, mode: Mode) -> io::Result<()> {
    mode.explicit_bits(FILE_BITS)
        .map_or(Ok(()), |bits| file.set_permissions(Permissions::from_mode(bits)))
}

/// Sets the permission bits of `file` if `mode` is [`Mode::Exact`].
pub(super) fn set_exact_mode(file: &fs::File, mode: Mode) -> io::Result<()> {
    match mode {
        Mode::Exact(bits) => file.set_permissions(Permissions::from_mode(bits)),
        Mode::Umask | Mode::HostUmask => Ok(()),
    }
}

/// Returns whether `metadata` has the mode that `mode` requires of an existing
/// file.
pub(super) fn has_mode(metadata: &Metadata, mode: Mode) -> bool {
    match mode {
        Mode::Exact(bits) => metadata.mode() & 0o7777 == bits & 0o7777,
        Mode::Umask | Mode::HostUmask => true,
    }
}

/// Flushes `dir` to disk.
pub(super) fn sync_dir(dir: &Path) -> Result<()> {
    File::open(dir)?.sync_all().context(OP, dir)
}
