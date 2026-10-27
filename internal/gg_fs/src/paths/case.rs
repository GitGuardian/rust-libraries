use std::path::Path;

use crate::error::{Context, Result};
use crate::sys;

const OP: &str = "case_sensitivity";

/// Whether names that differ only in case refer to the same file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseSensitivity {
    /// `Readme` and `README` are different names.
    Sensitive,
    /// `Readme` and `README` are the same name.
    Insensitive,
}

impl Default for CaseSensitivity {
    /// Returns [`CaseSensitivity::Insensitive`] on Windows and
    /// [`CaseSensitivity::Sensitive`] elsewhere.
    fn default() -> Self {
        if cfg!(windows) { Self::Insensitive } else { Self::Sensitive }
    }
}

/// Returns the case sensitivity of the file system at `path`.
///
/// This function accesses the filesystem.
///
/// # Errors
///
/// This function will return an error if `path` does not exist.
///
/// # Platform-specific behavior
///
/// The answer is per volume on macOS and per directory on Linux and Windows.
pub fn case_sensitivity(path: impl AsRef<Path>) -> Result<CaseSensitivity> {
    let path = path.as_ref();
    sys::case_sensitive(path).context(OP, path).map(|sensitive| {
        if sensitive { CaseSensitivity::Sensitive } else { CaseSensitivity::Insensitive }
    })
}
