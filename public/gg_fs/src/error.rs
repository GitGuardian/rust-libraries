use std::io;
use std::path::{Path, PathBuf};
use std::str::Utf8Error;
use std::time::Duration;

/// A specialized [`Result`](std::result::Result) type for filesystem
/// operations.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An error from a filesystem operation.
///
/// An `Error` records the operation that failed, the path it was applied to,
/// and the [`Cause`]. It is displayed as `<operation> <path>: <cause>`.
#[derive(Debug, thiserror::Error)]
#[error("{op} {}: {cause}", path.display())]
pub struct Error {
    op: &'static str,
    path: PathBuf,
    #[source]
    cause: Cause,
}

/// The cause of an [`Error`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Cause {
    /// An I/O error.
    #[error("{0}")]
    Io(#[from] io::Error),
    /// The file is larger than the size cap.
    #[error("{size} bytes exceeds the {cap} byte limit")]
    TooLarge {
        /// The size of the file, in bytes.
        size: u64,
        /// The size cap, in bytes.
        cap: u64,
    },
    /// The contents are not valid UTF-8.
    #[error("not valid UTF-8")]
    NotUtf8(#[from] Utf8Error),
    /// The contents could not be parsed.
    #[error("{0}")]
    Parse(#[source] Box<dyn std::error::Error + Send + Sync>),
    /// The path is a symbolic link, directory or other special file, and a
    /// regular file is required.
    #[error("is not a regular file")]
    NotRegularFile,
    /// The file does not meet the requested [`Trust`](crate::Trust).
    #[error("not trusted: {0}")]
    Untrusted(UntrustedReason),
    /// The file changed while it was open, or no longer matches the content an
    /// edit was computed against.
    #[error("changed unexpectedly")]
    Changed,
    /// The lock was not acquired before the timeout expired.
    #[error("lock not acquired within {0:?}")]
    Timeout(Duration),
    /// The path is not allowed: it is absolute, contains `..`, leaves its base
    /// through a symbolic link, or exceeds a length limit.
    #[error("invalid path: {0}")]
    InvalidPath(&'static str),
    /// The home directory could not be determined.
    #[error("no home directory could be determined")]
    NoHome,
}

/// Why a file does not meet the requested [`Trust`](crate::Trust).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum UntrustedReason {
    /// The owner is not an accepted user.
    #[error("not owned by an accepted user")]
    Owner,
    /// The file is writable by group or others.
    #[error("writable by group or others")]
    Writable,
}

impl Error {
    /// Creates an error for the operation `op` applied to `path`.
    pub(crate) fn new(op: &'static str, path: impl Into<PathBuf>, cause: impl Into<Cause>) -> Self {
        Self { op, path: path.into(), cause: cause.into() }
    }

    /// Creates an I/O error of `kind` with `message` for the operation `op`
    /// applied to `path`.
    pub(crate) fn with_kind(
        op: &'static str,
        path: impl Into<PathBuf>,
        kind: io::ErrorKind,
        message: &'static str,
    ) -> Self {
        Self::new(op, path, io::Error::new(kind, message))
    }

    /// Returns the name of the operation that failed.
    pub fn op(&self) -> &'static str {
        self.op
    }

    /// Returns the path the operation was applied to.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the cause of the error.
    pub fn cause(&self) -> &Cause {
        &self.cause
    }

    /// Returns the corresponding [`io::ErrorKind`] for this error.
    ///
    /// Causes that are not I/O errors map to the closest kind.
    pub fn kind(&self) -> io::ErrorKind {
        match &self.cause {
            Cause::Io(err) => err.kind(),
            Cause::TooLarge { .. } => io::ErrorKind::FileTooLarge,
            Cause::NotUtf8(_) | Cause::Parse(_) => io::ErrorKind::InvalidData,
            Cause::NotRegularFile | Cause::InvalidPath(_) => io::ErrorKind::InvalidInput,
            Cause::Untrusted(_) => io::ErrorKind::PermissionDenied,
            Cause::Timeout(_) => io::ErrorKind::TimedOut,
            Cause::Changed | Cause::NoHome => io::ErrorKind::Other,
        }
    }
}

/// Attaches the operation and path to a failed result.
pub(crate) trait Context<T> {
    /// Converts the error into an [`Error`] for the operation `op` applied to
    /// `path`.
    fn context(self, op: &'static str, path: &Path) -> Result<T>;
}

impl<T, E: Into<Cause>> Context<T> for std::result::Result<T, E> {
    fn context(self, op: &'static str, path: &Path) -> Result<T> {
        self.map_err(|cause| Error::new(op, path, cause))
    }
}

/// Extension methods for [`Result`].
pub trait ResultExt<T> {
    /// Converts an error into `None`, logging it at debug level.
    fn ok_or_log(self) -> Option<T>;
}

impl<T> ResultExt<T> for Result<T> {
    fn ok_or_log(self) -> Option<T> {
        self.map_err(|err| tracing::debug!("{err}")).ok()
    }
}
