//! Temporary files and directories.

use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, io, process};

use crate::dir::create_dir_all;
use crate::error::{Cause, Context, Error, Result, ResultExt};
use crate::file::File;
use crate::metadata::{NO_FOLLOW, metadata_if_exists};
use crate::paths::join;

const OP: &str = "temp";
const MAX_NAME_ATTEMPTS: u32 = 64;

static NAME_COUNTER: AtomicU64 = AtomicU64::new(0);

#[cfg_attr(unix, path = "unix.rs")]
#[cfg_attr(windows, path = "windows.rs")]
mod imp;

/// A directory in the temporary root, removed with its contents when dropped.
///
/// The directory is also removed if a panic unwinds through it. Use
/// [`TempDir::into_path`] to keep it.
///
/// # Platform-specific behavior
///
/// On Unix the root is `gg_fs-<uid>` in the system temporary directory, with
/// mode `0o700`. Creation fails if the root is a symbolic link or is owned by
/// another user. A root with broader permissions is reset to `0o700`. On
/// Windows the root is a `gg_fs` subdirectory of the system temporary
/// directory.
#[must_use = "the directory is removed when this is dropped"]
pub struct TempDir {
    path: PathBuf,
    keep: bool,
}

impl TempDir {
    /// Creates a directory named `name` in the temporary root.
    ///
    /// An existing directory with the same name is removed first.
    ///
    /// # Errors
    ///
    /// This function will return [`Cause::InvalidPath`] if `name` is not a
    /// single file name accepted by [`join`]. It will return an error if the
    /// root or the directory cannot be created.
    pub fn new(name: &str) -> Result<Self> {
        let root = imp::root()?;
        let path = child(&root, name)?;
        remove_existing(&path)?;
        create_dir_all(&path)?;
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

/// A file in the temporary root, removed when dropped.
///
/// The file is created exclusively, with mode `0o600` on Unix, and is open for
/// reading and writing. This type dereferences to [`File`].
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
    /// the root or the file cannot be created.
    pub fn new(prefix: &str) -> Result<Self> {
        let root = imp::root()?;
        let file = create_unique(&root, prefix, |name| {
            let path = child(&root, name)?;
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

fn child(root: &Path, name: &str) -> Result<PathBuf> {
    let path = join(root, name)?;
    if path.parent() == Some(root) {
        Ok(path)
    } else {
        Err(Error::new(OP, root, Cause::InvalidPath("is not a single file name")))
    }
}

fn remove_existing(path: &Path) -> Result<()> {
    match metadata_if_exists(path, &NO_FOLLOW)? {
        Some(metadata) => {
            let removed =
                if metadata.is_dir() { fs::remove_dir_all(path) } else { fs::remove_file(path) };
            removed.context(OP, path)
        }
        None => Ok(()),
    }
}
