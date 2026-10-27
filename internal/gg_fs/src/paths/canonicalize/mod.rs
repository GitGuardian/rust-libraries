use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{Context, Result};

const OP: &str = "canonicalize";

#[cfg_attr(unix, path = "unix.rs")]
#[cfg_attr(windows, path = "windows.rs")]
mod imp;

/// Returns the canonical, absolute form of a path with all intermediate
/// components normalized and symbolic links resolved.
///
/// # Errors
///
/// This function will return an error if `path` does not exist, or a non-final
/// component is not a directory.
///
/// # Platform-specific behavior
///
/// On Windows the `\\?\` prefix is removed where the path can be written
/// without it.
pub fn canonicalize(path: impl AsRef<Path>) -> Result<PathBuf> {
    let path = path.as_ref();
    fs::canonicalize(path).map(imp::simplify).context(OP, path)
}
