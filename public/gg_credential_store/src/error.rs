use std::path::{Path, PathBuf};
use std::{fmt, io};

use chrono::{DateTime, Utc};
use gg_url::DashboardUrl;

use crate::SkipReason;

/// An error from a [`CredentialStore`](crate::CredentialStore) operation.
///
/// It records the instance and the file where it applies, and the [`Cause`].
#[derive(Debug)]
pub struct CredentialStoreError {
    instance: Option<Box<DashboardUrl>>,
    path: Option<PathBuf>,
    cause: Cause,
}

/// The cause of a [`CredentialStoreError`].
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Cause {
    /// The machine has no machine token for the instance, the request has
    /// no explicit token, and no user token is stored.
    #[error("no token is available")]
    NoToken,
    /// The machine token was not returned, for the given reason, and no user
    /// token is stored.
    #[error("{0}, and no user token is stored")]
    MachineTokenSkipped(SkipReason),
    /// The OS store denies this installation access to the token.
    #[error("the OS store denies this installation access to the token")]
    OsStoreDenied,
    /// The OS store cannot be used, for the given reason.
    #[error("the OS store is unavailable: {0}")]
    OsStoreUnavailable(String),
    /// The OS store is locked.
    #[error("the OS store is locked: {0}")]
    OsStoreLocked(String),
    /// The config file points to the OS store, and the OS store has no token.
    #[error("the token is missing from the OS store")]
    OsStoreItemMissing,
    /// The token is past its expiry date.
    #[error("the token expired on {0}")]
    Expired(DateTime<Utc>),
    /// The caller may not write the machine store.
    #[error("no permission to write the machine store")]
    MachineStoreDenied,
    /// A machine token was stored without a machine id.
    #[error("a machine token needs a machine id")]
    MissingMachineId,
    /// No home directory could be determined, so the config file cannot be
    /// found.
    #[error("no home directory could be determined")]
    NoHome,
    /// The file exists and cannot be read.
    #[error("cannot read: {0}")]
    Read(io::Error),
    /// The file cannot be written.
    #[error("cannot write: {0}")]
    Write(io::Error),
    /// The file is not in the expected format.
    #[error("malformed: {0}")]
    Malformed(String),
}

impl CredentialStoreError {
    pub(crate) fn at(path: &Path, cause: Cause) -> Self {
        Self { instance: None, path: Some(path.to_path_buf()), cause }
    }

    pub(crate) fn for_instance(instance: &DashboardUrl, cause: Cause) -> Self {
        Self { instance: Some(Box::new(instance.clone())), path: None, cause }
    }

    pub(crate) fn read(err: gg_fs::Error) -> Self {
        match err.cause() {
            gg_fs::Cause::NoHome => Self { instance: None, path: None, cause: Cause::NoHome },
            gg_fs::Cause::NotUtf8(utf8) => Self::at(err.path(), Cause::Malformed(utf8.to_string())),
            cause => {
                Self::at(err.path(), Cause::Read(io::Error::new(err.kind(), cause.to_string())))
            }
        }
    }

    pub(crate) fn write(err: gg_fs::Error) -> Self {
        Self::at(err.path(), Cause::Write(io::Error::new(err.kind(), err.cause().to_string())))
    }

    /// Returns the instance the operation was applied to, if any.
    pub fn instance(&self) -> Option<&DashboardUrl> {
        self.instance.as_deref()
    }

    /// Returns the file the error is about, if any.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Returns the cause of the error.
    pub fn cause(&self) -> &Cause {
        &self.cause
    }
}

impl fmt::Display for CredentialStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(instance) = &self.instance {
            write!(f, "{instance}: ")?;
        }
        if let Some(path) = &self.path {
            write!(f, "{}: ", path.display())?;
        }
        write!(f, "{}", self.cause)
    }
}

impl std::error::Error for CredentialStoreError {}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::*;

    /// GIVEN an error that cannot read a file
    /// WHEN it is formatted and its chain of sources is followed
    /// THEN the message names the file and the cause once, and there is no
    /// source
    #[test]
    fn message_holds_the_whole_chain() {
        let io = io::Error::new(io::ErrorKind::PermissionDenied, "denied");
        let err = CredentialStoreError::at(Path::new("auth_config.yaml"), Cause::Read(io));

        assert_eq!(err.to_string(), "auth_config.yaml: cannot read: denied");
        assert!(err.source().is_none());
        assert!(err.cause().source().is_none());
    }
}
