#[cfg(any(target_os = "linux", windows))]
use std::path::Path;
#[cfg(any(target_os = "linux", windows))]
use std::{fs, io};

#[cfg(any(target_os = "linux", windows))]
use crate::paths::parent_dir;

/// Identifies a file. Two ids are equal if and only if they refer to the same
/// file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FileId {
    volume: u64,
    index: u128,
}

#[cfg(any(target_os = "linux", windows))]
fn containing_dir(path: &Path) -> io::Result<&Path> {
    if fs::metadata(path)?.is_dir() { Ok(path) } else { Ok(parent_dir(path)) }
}

#[cfg_attr(target_os = "linux", path = "linux.rs")]
#[cfg_attr(target_os = "macos", path = "macos.rs")]
#[cfg_attr(windows, path = "windows.rs")]
mod imp;
#[cfg(unix)]
mod unix;

pub(crate) use imp::*;
