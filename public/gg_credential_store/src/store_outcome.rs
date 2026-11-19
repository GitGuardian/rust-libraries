use std::path::PathBuf;

use crate::Cause;

/// Where [`CredentialStore::store`](crate::CredentialStore::store) put a token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreOutcome {
    /// The user's credential store of the OS.
    UserOsStore,
    /// `auth_config.yaml`, in clear text.
    UserFile {
        /// Why the token is not in the user's OS store.
        reason: CleartextReason,
        /// The path of the file the token was written to.
        path: PathBuf,
    },
    /// The machine-wide credential store of the OS.
    MachineOsStore,
    /// `machine_credential.yaml`, in clear text.
    MachineFile {
        /// Why the token is not in the machine-wide OS store.
        reason: CleartextReason,
        /// The path of the file the token was written to.
        path: PathBuf,
    },
}

/// Why a token was stored in clear text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CleartextReason {
    /// `GGSHIELD_NO_KEYRING` is set.
    Disabled,
    /// This caller does not use the OS store, for example because it is not
    /// signed by GitGuardian on macOS.
    UnsupportedCaller,
    /// No OS store can hold the token, for the given reason.
    Unavailable(String),
    /// The OS store is locked.
    Locked,
    /// The OS store refused the token, for the given reason.
    WriteFailed(String),
}

impl CleartextReason {
    pub(crate) fn refused(cause: Cause) -> Self {
        match cause {
            Cause::OsStoreLocked(_) => Self::Locked,
            cause => Self::WriteFailed(cause.to_string()),
        }
    }
}
