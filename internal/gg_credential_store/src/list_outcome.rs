use std::path::PathBuf;

use crate::StoredToken;

/// What [`CredentialStore::list`](crate::CredentialStore::list) found on the
/// device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListOutcome {
    /// The stored tokens, without their secrets. The machine token is listed
    /// first.
    pub tokens: Vec<StoredToken>,
    /// The entries that cannot be read.
    pub unreadable: Vec<UnreadableEntry>,
}

/// A stored token that cannot be read.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum UnreadableEntry {
    /// An instance of `auth_config.yaml` whose URL is malformed.
    UserInstance {
        /// The URL as written in the file.
        url: String,
        /// Why the URL cannot be read.
        reason: String,
    },
    /// `machine_credential.yaml`, which cannot be used.
    MachineFile {
        /// The path of the file.
        path: PathBuf,
        /// Why the file cannot be used.
        reason: String,
    },
}
