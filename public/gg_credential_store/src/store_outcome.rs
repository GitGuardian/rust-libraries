use std::path::PathBuf;

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
    /// `machine_credential.yaml`.
    MachineFile,
}

/// Why a user token was stored in clear text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleartextReason {
    /// `GGSHIELD_NO_KEYRING` is set.
    Disabled,
    /// This caller does not use the OS store, for example because it is not
    /// signed by GitGuardian on macOS.
    UnsupportedCaller,
    /// No OS store is reachable.
    Unavailable,
    /// The OS store is locked.
    Locked,
    /// The OS store refused the token.
    WriteFailed,
}
