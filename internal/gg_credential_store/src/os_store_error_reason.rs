use crate::Cause;

/// Why the OS store could not hold or remove a token.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OsStoreErrorReason {
    /// `GGSHIELD_NO_KEYRING` is set.
    Disabled,
    /// This caller does not use the OS store, for example because it is not
    /// signed by GitGuardian on macOS.
    UnsupportedCaller,
    /// The OS store cannot be used, for the given reason.
    Unavailable(String),
    /// The OS store is locked.
    Locked,
    /// The OS store refused the operation, for the given reason.
    Refused(String),
}

impl OsStoreErrorReason {
    pub(crate) fn refused(cause: Cause) -> Self {
        match cause {
            Cause::OsStoreLocked(_) => Self::Locked,
            cause => Self::Refused(cause.to_string()),
        }
    }
}
