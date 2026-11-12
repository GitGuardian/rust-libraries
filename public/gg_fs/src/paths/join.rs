use std::io;
use std::path::{Path, PathBuf};

use super::canonicalize::canonicalize;
use super::within::is_within;
use crate::error::{Cause, Error, Result};
use crate::metadata::{NO_FOLLOW, metadata_with};

const OP: &str = "join";
const MAX_NAME_CHARS: usize = 255;
const MAX_PATH_CHARS: usize = 4096;

/// Joins `relative` onto `base`, keeping the result below `base`.
///
/// This function accesses the filesystem to resolve symbolic links. A link
/// created afterwards is not detected.
///
/// # Errors
///
/// This function will return [`Cause::InvalidPath`]
/// if `relative` is absolute, contains a `..` component, leaves `base` through a
/// symbolic link or passes through a dangling one, has a name
/// longer than 255 characters, or makes the path longer than 4096 characters.
/// Both `/` and `\` separate components. It will return an I/O error if `base`
/// does not exist.
pub fn join(base: impl AsRef<Path>, relative: &str) -> Result<PathBuf> {
    let base = base.as_ref();
    if is_absolute(relative) {
        Err(Error::new(OP, base, Cause::InvalidPath("is absolute")))
    } else {
        let mut joined = base.to_path_buf();
        for name in relative.split(['/', '\\']).filter(|name| !matches!(*name, "" | ".")) {
            check_name(name).map_err(|reason| Error::new(OP, base, Cause::InvalidPath(reason)))?;
            joined.push(name);
        }
        if joined.to_string_lossy().chars().count() > MAX_PATH_CHARS {
            Err(Error::new(OP, base, Cause::InvalidPath("path is too long")))
        } else if stays_below(base, &joined)? {
            Ok(joined)
        } else {
            let reason = "leaves the base through a symbolic link";
            Err(Error::new(OP, base, Cause::InvalidPath(reason)))
        }
    }
}

fn is_absolute(relative: &str) -> bool {
    relative.starts_with(['/', '\\'])
        || matches!(relative.as_bytes(), [drive, b':'] | [drive, b':', b'/' | b'\\', ..] if drive.is_ascii_alphabetic())
}

fn check_name(name: &str) -> Result<(), &'static str> {
    if name == ".." {
        Err("contains `..`")
    } else if name.chars().count() > MAX_NAME_CHARS {
        Err("name is too long")
    } else if name.contains(char::is_control) {
        Err("contains a control character")
    } else if cfg!(windows) && !is_windows_file_name(name) {
        Err("is not a valid Windows file name")
    } else {
        Ok(())
    }
}

fn is_windows_file_name(name: &str) -> bool {
    let stem = name.split('.').next().unwrap_or_default().trim_end().to_ascii_uppercase();
    let is_device = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || matches!(stem.strip_prefix("COM").or_else(|| stem.strip_prefix("LPT")), Some(n) if matches!(n.as_bytes(), [b'1'..=b'9']));
    !name.contains([':', '<', '>', '"', '|', '?', '*']) && !name.ends_with(['.', ' ']) && !is_device
}

fn stays_below(base: &Path, joined: &Path) -> Result<bool> {
    let root = canonicalize(base)?;
    match nearest_existing(joined)? {
        Some(ancestor) => match canonicalize(ancestor) {
            Ok(resolved) => Ok(is_within(&root, resolved)),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(err) => Err(err),
        },
        None => Ok(true),
    }
}

fn nearest_existing(path: &Path) -> Result<Option<&Path>> {
    for ancestor in path.ancestors() {
        match metadata_with(ancestor, &NO_FOLLOW) {
            Ok(_) => return Ok(Some(ancestor)),
            Err(err) if is_missing(&err) => {}
            Err(err) => return Err(err),
        }
    }
    Ok(None)
}

fn is_missing(err: &Error) -> bool {
    matches!(err.kind(), io::ErrorKind::NotFound | io::ErrorKind::NotADirectory)
}
