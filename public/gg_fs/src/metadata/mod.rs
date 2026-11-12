use std::path::Path;
use std::time::UNIX_EPOCH;
use std::{fs, io};

use crate::error::{Context, Result};

const OP: &str = "metadata";

pub use std::fs::{FileType, Metadata};

#[cfg_attr(unix, path = "unix.rs")]
#[cfg_attr(windows, path = "windows.rs")]
mod imp;

pub(crate) use imp::{hard_links, mode};

pub(crate) const NO_FOLLOW: MetadataOptions = MetadataOptions { follow_symlinks: false };

/// Options for querying metadata with [`metadata_with`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataOptions {
    /// Whether to follow symbolic links. When `false`, the metadata describes
    /// a symbolic link itself instead of its target. Defaults to `true`.
    pub follow_symlinks: bool,
}

impl Default for MetadataOptions {
    /// Returns options that follow symbolic links.
    fn default() -> Self {
        Self { follow_symlinks: true }
    }
}

/// Queries the metadata of a file, following symbolic links.
///
/// This is [`metadata_with`] with default [`MetadataOptions`].
///
/// # Errors
///
/// This function will return an error in the situations of [`metadata_with`].
pub fn metadata(path: impl AsRef<Path>) -> Result<Metadata> {
    metadata_with(path, &MetadataOptions::default())
}

/// Queries the metadata of a file.
///
/// If `path` is a symbolic link and `options.follow_symlinks` is `false`, the
/// metadata describes the link itself instead of its target.
///
/// # Errors
///
/// This function will return an error if `path` does not exist, or the user
/// lacks permission to search a parent directory.
pub fn metadata_with(path: impl AsRef<Path>, options: &MetadataOptions) -> Result<Metadata> {
    let path = path.as_ref();
    let metadata =
        if options.follow_symlinks { fs::metadata(path) } else { fs::symlink_metadata(path) };
    metadata.context(OP, path)
}

/// Queries the metadata of a file, returning `None` if it does not exist.
pub(crate) fn metadata_if_exists(
    path: impl AsRef<Path>,
    options: &MetadataOptions,
) -> Result<Option<Metadata>> {
    match metadata_with(path, options) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err),
    }
}

/// Extension methods for [`Metadata`].
pub trait MetadataExt {
    /// Returns the modification time as nanoseconds since the Unix epoch,
    /// negative before it.
    ///
    /// Returns `None` if the time is unavailable or does not fit in an `i64`.
    fn modified_nanos(&self) -> Option<i64>;
}

impl MetadataExt for Metadata {
    fn modified_nanos(&self) -> Option<i64> {
        let modified = self.modified().ok()?;
        match modified.duration_since(UNIX_EPOCH) {
            Ok(since) => i64::try_from(since.as_nanos()).ok(),
            Err(before) => i64::try_from(before.duration().as_nanos()).ok().map(|nanos| -nanos),
        }
    }
}
