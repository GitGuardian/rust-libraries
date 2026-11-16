//! Temporary files and directories.

use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, fs, io, process};

use crate::error::{Cause, Context, Error, Result, ResultExt};
use crate::file::File;
use crate::paths::join;

const OP: &str = "temp";
const MAX_NAME_ATTEMPTS: u32 = 64;

static NAME_COUNTER: AtomicU64 = AtomicU64::new(0);

#[cfg_attr(unix, path = "unix.rs")]
#[cfg_attr(windows, path = "windows.rs")]
mod imp;

/// A directory in the system temporary directory, removed with its contents
/// when dropped.
///
/// The directory is also removed if a panic unwinds through it. Use
/// [`TempDir::into_path`] to keep it.
///
/// # Platform-specific behavior
///
/// On Unix the directory has mode `0o700`. On Windows only the current user
/// has access to it and to the entries created inside it.
#[derive(Debug)]
#[must_use = "the directory is removed when this is dropped"]
pub struct TempDir {
    path: PathBuf,
    keep: bool,
}

impl TempDir {
    /// Creates a directory whose name starts with `prefix`.
    ///
    /// # Errors
    ///
    /// This function will return [`Cause::InvalidPath`] if `prefix` does not
    /// start a single file name accepted by [`join`]. It will return an error if
    /// the directory cannot be created.
    pub fn new(prefix: &str) -> Result<Self> {
        let temp = env::temp_dir();
        let path = create_unique(&temp, prefix, |name| {
            let path = child(&temp, name)?;
            imp::create_dir(&path)?;
            Ok(path)
        })?;
        Ok(Self { path, keep: false })
    }

    /// Returns the path of the directory.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Consumes the guard without removing the directory, and returns its path.
    pub fn into_path(mut self) -> PathBuf {
        self.keep = true;
        std::mem::take(&mut self.path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        if !self.keep {
            fs::remove_dir_all(&self.path).context(OP, &self.path).ok_or_log();
        }
    }
}

/// A file in the system temporary directory, removed when dropped.
///
/// The file is created exclusively and is open for reading and writing. This
/// type dereferences to [`File`].
///
/// # Platform-specific behavior
///
/// On Unix the file has mode `0o600`. On Windows only the current user has
/// access to it.
#[derive(Debug)]
#[must_use = "the file is removed when this is dropped"]
pub struct TempFile {
    file: File,
}

impl TempFile {
    /// Creates a file whose name starts with `prefix`.
    ///
    /// # Errors
    ///
    /// This function will return [`Cause::InvalidPath`] if `prefix` does not
    /// start a single file name accepted by [`join`]. It will return an error if
    /// the file cannot be created.
    pub fn new(prefix: &str) -> Result<Self> {
        let temp = env::temp_dir();
        let file = create_unique(&temp, prefix, |name| {
            let path = child(&temp, name)?;
            let file = imp::create_file(&path).context(OP, &path)?;
            Ok(File::from_std(file, path))
        })?;
        Ok(Self { file })
    }

    /// Returns the path of the file.
    pub fn path(&self) -> &Path {
        self.file.path()
    }
}

impl Deref for TempFile {
    type Target = File;
    fn deref(&self) -> &File {
        &self.file
    }
}
impl DerefMut for TempFile {
    fn deref_mut(&mut self) -> &mut File {
        &mut self.file
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let path = self.file.path();
        fs::remove_file(path).context(OP, path).ok_or_log();
    }
}

/// Creates a file with the closure `create`, trying names that start with
/// `prefix` until one is unused.
pub(crate) fn create_unique<T>(
    dir: &Path,
    prefix: &str,
    create: impl Fn(&str) -> Result<T>,
) -> Result<T> {
    let pid = process::id();
    for _ in 0..MAX_NAME_ATTEMPTS {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos());
        let count = NAME_COUNTER.fetch_add(1, Ordering::Relaxed);
        match create(&format!("{prefix}{pid}-{nanos}-{count}")) {
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {}
            created => return created,
        }
    }
    Err(Error::with_kind(OP, dir, io::ErrorKind::AlreadyExists, "no unused file name"))
}

fn child(dir: &Path, name: &str) -> Result<PathBuf> {
    let path = join(dir, name)?;
    if path.parent() == Some(dir) {
        Ok(path)
    } else {
        Err(Error::new(OP, dir, Cause::InvalidPath("is not a single file name")))
    }
}
