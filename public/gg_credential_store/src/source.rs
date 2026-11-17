use crate::SkipReason;

/// The origin of a token returned by
/// [`CredentialStore::read`](crate::CredentialStore::read).
///
/// A caller sends the machine id of a machine token in place of its own.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Source {
    /// The machine token, from the machine-wide credential store of the OS.
    MachineOsStore {
        /// The machine id recorded with the token.
        machine_id: String,
    },
    /// The machine token, from `machine_credential.yaml`.
    MachineFile {
        /// The machine id recorded with the token.
        machine_id: String,
    },
    /// The explicit token of the request.
    Explicit,
    /// The user token, from the user's credential store of the OS.
    UserOsStore {
        /// Why the machine token of the instance was not returned, if the
        /// machine has one.
        machine_token_skipped: Option<SkipReason>,
    },
    /// The user token, in clear text in `auth_config.yaml`.
    UserFile {
        /// Why the machine token of the instance was not returned, if the
        /// machine has one.
        machine_token_skipped: Option<SkipReason>,
    },
}
