#[cfg(any(test, feature = "in-memory-store"))]
mod in_memory;

use std::ffi::OsStr;

#[cfg(any(test, feature = "in-memory-store"))]
pub use in_memory::{InMemoryOsStore, OsStoreFailure};

use crate::{Cause, CleartextReason};

#[allow(dead_code, reason = "current() reads it once a platform store exists")]
const NO_KEYRING_VAR: &str = "GGSHIELD_NO_KEYRING";

/// Whose token an operation concerns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Scope {
    User,
    #[allow(dead_code, reason = "the machine store task reads and writes machine tokens")]
    Machine,
}

/// The credential store of the operating system.
#[derive(Debug)]
pub(crate) enum OsStore {
    #[cfg(any(test, feature = "in-memory-store"))]
    InMemory(InMemoryOsStore),
}

#[cfg_attr(not(any(test, feature = "in-memory-store")), allow(unused_variables))]
impl OsStore {
    /// Returns the OS store of this platform, or `None` if it has none.
    /// `GGSHIELD_NO_KEYRING` turns off the user scope only.
    pub(crate) fn current() -> Option<Self> {
        None
    }

    /// Returns the reason a token of `scope` cannot be stored in this store, if
    /// any.
    pub(crate) fn check_available(&self, scope: Scope) -> Result<(), CleartextReason> {
        match *self {
            #[cfg(any(test, feature = "in-memory-store"))]
            Self::InMemory(ref store) => store.check_available(scope),
        }
    }

    /// Returns the token of `scope` stored for `account`, or `None` if there is
    /// none.
    pub(crate) fn read(&self, scope: Scope, account: &str) -> Result<Option<String>, Cause> {
        match *self {
            #[cfg(any(test, feature = "in-memory-store"))]
            Self::InMemory(ref store) => store.read(scope, account),
        }
    }

    /// Stores `token` of `scope` for `account`. With `interactive`, the store
    /// may ask the user.
    pub(crate) fn write(
        &self,
        scope: Scope,
        account: &str,
        token: &str,
        interactive: bool,
    ) -> Result<(), Cause> {
        match *self {
            #[cfg(any(test, feature = "in-memory-store"))]
            Self::InMemory(ref store) => store.write(scope, account, token, interactive),
        }
    }

    /// Removes the user token of `account`. Removing a missing token succeeds.
    pub(crate) fn delete(&self, account: &str) -> Result<(), Cause> {
        match *self {
            #[cfg(any(test, feature = "in-memory-store"))]
            Self::InMemory(ref store) => store.delete(account),
        }
    }

    /// Returns whether `no_keyring`, the value of `GGSHIELD_NO_KEYRING`,
    /// turns off the user scope. Every value except `false` and `0`, in any
    /// case, turns it off. A value that is not valid UTF-8 does not.
    #[allow(dead_code, reason = "current() reads it once a platform store exists")]
    fn user_scope_disabled(no_keyring: Option<&OsStr>) -> bool {
        no_keyring
            .and_then(OsStr::to_str)
            .is_some_and(|value| !matches!(value.to_lowercase().as_str(), "false" | "0"))
    }
}

#[cfg(test)]
mod tests {
    use yare::parameterized;

    use super::*;

    /// GIVEN a value of `GGSHIELD_NO_KEYRING`
    /// WHEN it is read
    /// THEN only `false` and `0`, in any case, keep the user scope enabled
    #[parameterized(
        unset = { None, false },
        one = { Some("1"), true },
        empty = { Some(""), true },
        word = { Some("yes"), true },
        false_lower = { Some("false"), false },
        false_upper = { Some("FALSE"), false },
        zero = { Some("0"), false },
    )]
    fn no_keyring_switch(value: Option<&str>, disabled: bool) {
        assert_eq!(OsStore::user_scope_disabled(value.map(OsStr::new)), disabled);
    }

    /// GIVEN a value of `GGSHIELD_NO_KEYRING` that is not valid UTF-8
    /// WHEN it is read
    /// THEN the user scope stays enabled
    #[cfg(unix)]
    #[test]
    fn no_keyring_switch_ignores_non_utf8() {
        use std::os::unix::ffi::OsStrExt;

        assert!(!OsStore::user_scope_disabled(Some(OsStr::from_bytes(b"\xff"))));
    }
}
