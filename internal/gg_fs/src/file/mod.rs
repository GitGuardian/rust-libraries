use std::io;
use std::ops::Deref;
use std::path::{Path, PathBuf};

use crate::error::{Context, Error, Result};
use crate::metadata::Metadata;
use crate::sys;

const OP: &str = "open";
const STAT: &str = "metadata";

#[cfg_attr(target_os = "linux", path = "linux.rs")]
#[cfg_attr(target_os = "macos", path = "macos.rs")]
#[cfg_attr(windows, path = "windows.rs")]
mod imp;
#[cfg(unix)]
mod unix;

fn std_options(access: Access) -> std::fs::OpenOptions {
    let mut options = std::fs::OpenOptions::new();
    match access {
        Access::Read => options.read(true),
        Access::ReadWrite => options.read(true).write(true),
        Access::Append => options.append(true).create(true),
    };
    options
}

/// Options for opening a [`File`] with [`File::open_with`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenOptions {
    /// Whether to follow symbolic links. When `false`, opening a symbolic link
    /// or reparse point fails with
    /// [`Cause::NotRegularFile`](crate::Cause::NotRegularFile). Defaults to
    /// `true`.
    pub follow_symlinks: bool,
    /// Whether to avoid caching the contents of the file, where supported.
    /// Failure to do so is ignored. Defaults to `false`.
    pub no_cache: bool,
    /// How the file is accessed. Defaults to [`Access::Read`].
    pub access: Access,
}

/// How a file is accessed when it is opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Access {
    /// Read only.
    #[default]
    Read,
    /// Read and write in place, without creating or truncating the file.
    ReadWrite,
    /// Append, creating the file if it does not exist. The file cannot be read.
    Append,
}

impl Default for OpenOptions {
    /// Returns options that follow symbolic links and open the file read-only.
    fn default() -> Self {
        Self { follow_symlinks: true, no_cache: false, access: Access::Read }
    }
}

/// An open file that remembers its path.
///
/// This type dereferences to [`std::fs::File`]. Errors returned by its own
/// methods carry the path.
#[derive(Debug)]
pub struct File {
    inner: std::fs::File,
    path: PathBuf,
}

impl File {
    /// Opens a file in read-only mode.
    ///
    /// This is [`File::open_with`] with default [`OpenOptions`].
    ///
    /// # Errors
    ///
    /// This function will return an error if `path` does not exist, or the user
    /// lacks permission to open it.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_with(path, &OpenOptions::default())
    }

    /// Opens a file with the given options.
    ///
    /// # Errors
    ///
    /// This function will return an error in the situations of [`File::open`],
    /// and [`Cause::NotRegularFile`](crate::Cause::NotRegularFile) if `path` is
    /// a FIFO, or if `options.follow_symlinks` is `false` and `path` is a
    /// symbolic link or reparse point.
    ///
    /// # Platform-specific behavior
    ///
    /// On Windows other processes can read, write and delete the file while it
    /// is open.
    pub fn open_with(path: impl AsRef<Path>, options: &OpenOptions) -> Result<Self> {
        let path = path.as_ref();
        let inner = imp::open(path, options)?;
        Ok(Self { inner, path: path.to_path_buf() })
    }

    pub(crate) fn from_std(inner: std::fs::File, path: PathBuf) -> Self {
        Self { inner, path }
    }

    /// Returns the path the file was opened with.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Queries metadata about the open file.
    ///
    /// The result describes the file that was opened, even if the path has
    /// since been replaced.
    ///
    /// # Errors
    ///
    /// This function will return an error if the metadata cannot be read.
    pub fn metadata(&self) -> Result<Metadata> {
        self.inner.metadata().context(STAT, &self.path)
    }

    /// Returns whether the path now refers to a different file than the one that
    /// was opened. A path that no longer exists has changed.
    pub(crate) fn has_path_changed(&self) -> Result<bool> {
        let opened = sys::file_id(&self.inner).context(STAT, &self.path)?;
        match sys::path_id(&self.path) {
            Ok(current) => Ok(opened != current),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(true),
            Err(err) => Err(Error::new(STAT, &self.path, err)),
        }
    }
}

impl Deref for File {
    type Target = std::fs::File;
    fn deref(&self) -> &std::fs::File {
        &self.inner
    }
}
impl From<File> for std::fs::File {
    fn from(file: File) -> Self {
        file.inner
    }
}
impl io::Read for File {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}
impl io::Write for File {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
impl io::Seek for File {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.inner.seek(pos)
    }
}
