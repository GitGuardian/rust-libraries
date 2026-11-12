use std::fs::Metadata;
use std::io;
use std::path::Path;

use crate::error::{Cause, Error, Result, UntrustedReason};
use crate::metadata::{MetadataOptions, NO_FOLLOW, metadata_if_exists, metadata_with};
use crate::mode::Mode;
use crate::trust;

const OP: &str = "create_dir_all";

#[cfg_attr(unix, path = "unix.rs")]
#[cfg_attr(windows, path = "windows.rs")]
mod imp;

#[cfg(unix)]
pub(crate) use imp::set_mode;

/// Options for [`create_dir_all_with`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirOptions {
    /// The mode of each directory created. Defaults to `Mode::Exact(0o700)`.
    pub mode: Mode,
}

impl Default for DirOptions {
    /// Returns options that create directories with mode `0o700`.
    fn default() -> Self {
        Self { mode: Mode::Exact(0o700) }
    }
}

/// Recursively creates a directory and all of its parent components if they are
/// missing.
///
/// Directories created have mode `0o700`. Existing directories are not
/// modified. A symbolic link at `path` is accepted if it points to a directory
/// owned by the calling user or root.
///
/// # Errors
///
/// This function will return an error if a component cannot be created, or if
/// `path` is a symbolic link that dangles or does not point to a directory. It
/// will return [`Cause::Untrusted`] if `path` is a symbolic link to a directory
/// owned by another user.
///
/// # Platform-specific behavior
///
/// On Windows new directories inherit the ACL of their parent.
pub fn create_dir_all(path: impl AsRef<Path>) -> Result<()> {
    create_dir_all_with(path, &DirOptions::default())
}

/// Recursively creates a directory and all of its parent components, with
/// options.
///
/// # Errors
///
/// This function will return an error in the situations of [`create_dir_all`].
pub fn create_dir_all_with(path: impl AsRef<Path>, options: &DirOptions) -> Result<()> {
    let path = path.as_ref();
    match metadata_if_exists(path, &NO_FOLLOW)? {
        Some(metadata) => require_dir(path, &metadata),
        None => create_missing(path, options.mode),
    }
}

fn require_dir(path: &Path, metadata: &Metadata) -> Result<()> {
    if metadata.is_dir() {
        Ok(())
    } else if !metadata.is_symlink() {
        Err(Error::with_kind(OP, path, io::ErrorKind::NotADirectory, "is not a directory"))
    } else {
        match metadata_if_exists(path, &MetadataOptions::default())? {
            None => Err(Error::with_kind(
                OP,
                path,
                io::ErrorKind::InvalidInput,
                "is a dangling symbolic link",
            )),
            Some(target) if target.is_dir() && !trust::is_owned_by_caller_or_root(path)? => {
                Err(Error::new(OP, path, Cause::Untrusted(UntrustedReason::Owner)))
            }
            Some(target) => require_dir(path, &target),
        }
    }
}

fn create_missing(path: &Path, mode: Mode) -> Result<()> {
    let missing: Vec<&Path> = path.ancestors().take_while(|dir| is_missing(dir)).collect();
    let mut created = Vec::with_capacity(missing.len());
    for dir in missing.into_iter().rev() {
        match imp::create_dir(dir, mode) {
            Ok(()) => created.push(dir),
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists && is_real_dir(dir) => {}
            Err(err) => return Err(Error::new(OP, dir, err)),
        }
    }
    created.into_iter().rev().try_for_each(|dir| imp::set_mode(dir, mode))
}

fn is_missing(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && metadata_if_exists(path, &NO_FOLLOW).is_ok_and(|metadata| metadata.is_none())
}

fn is_real_dir(path: &Path) -> bool {
    metadata_with(path, &NO_FOLLOW).is_ok_and(|metadata| metadata.is_dir())
}
