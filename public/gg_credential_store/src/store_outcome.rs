use std::path::PathBuf;

use crate::OsStoreErrorReason;

/// Where [`CredentialStore::store`](crate::CredentialStore::store) put a token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreOutcome {
    /// The user's credential store of the OS.
    UserOsStore,
    /// `auth_config.yaml`, in clear text.
    UserFile {
        /// Why the token is not in the user's OS store.
        reason: OsStoreErrorReason,
        /// The path of the file the token was written to.
        path: PathBuf,
    },
    /// The machine-wide credential store of the OS.
    MachineOsStore,
    /// `machine_credential.yaml`, in clear text.
    MachineFile {
        /// Why the token is not in the machine-wide OS store.
        reason: OsStoreErrorReason,
        /// The path of the file the token was written to.
        path: PathBuf,
    },
}
