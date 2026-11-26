use crate::OsStoreErrorReason;

/// What [`CredentialStore::delete`](crate::CredentialStore::delete) did with a
/// token.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DeleteOutcome {
    /// The token was removed.
    Removed,
    /// No token was stored for the instance.
    NotStored,
    /// The token was removed from `auth_config.yaml`, and a token may stay in
    /// the OS store, for the given reason.
    KeptInOsStore(OsStoreErrorReason),
}
