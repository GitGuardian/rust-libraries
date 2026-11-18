#[cfg(any(test, feature = "in-memory-store"))]
mod in_memory;
#[cfg(target_os = "macos")]
mod macos;

use std::ffi::OsStr;

#[cfg(any(test, feature = "in-memory-store"))]
pub use in_memory::{InMemoryOsStore, OsStoreFailure};
#[cfg(target_os = "macos")]
use macos::KeychainStore;

use crate::{Cause, CleartextReason};

#[cfg_attr(not(target_os = "macos"), allow(dead_code, reason = "no OS store on this platform"))]
const NO_KEYRING_VAR: &str = "GGSHIELD_NO_KEYRING";

/// Whose token an operation concerns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Scope {
    User,
    Machine,
}

/// The credential store of the operating system.
#[derive(Debug)]
pub(crate) enum OsStore {
    #[cfg(target_os = "macos")]
    Keychain(KeychainStore),
    #[cfg(any(test, feature = "in-memory-store"))]
    InMemory(InMemoryOsStore),
}

#[cfg_attr(
    not(any(test, feature = "in-memory-store", target_os = "macos")),
    allow(unused_variables)
)]
impl OsStore {
    /// Returns the OS store of this platform, or `None` if it has none or
    /// `GGSHIELD_NO_KEYRING` disables it.
    pub(crate) fn current() -> Option<Self> {
        #[cfg(target_os = "macos")]
        if !Self::disabled(std::env::var_os(NO_KEYRING_VAR).as_deref()) {
            return Some(Self::Keychain(KeychainStore::new()));
        }
        None
    }

    /// Returns the reason a token of `scope` cannot be stored in this store, if
    /// any.
    pub(crate) fn check_available(&self, scope: Scope) -> Result<(), CleartextReason> {
        match *self {
            #[cfg(target_os = "macos")]
            Self::Keychain(ref store) => store.check_available(scope),
            #[cfg(any(test, feature = "in-memory-store"))]
            Self::InMemory(ref store) => store.check_available(scope),
        }
    }

    /// Returns the token of `scope` stored for `account`, or `None` if there is
    /// none.
    pub(crate) fn read(&self, scope: Scope, account: &str) -> Result<Option<String>, Cause> {
        match *self {
            #[cfg(target_os = "macos")]
            Self::Keychain(ref store) => store.read(scope, account),
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
            #[cfg(target_os = "macos")]
            Self::Keychain(ref store) => store.write(scope, account, token, interactive),
            #[cfg(any(test, feature = "in-memory-store"))]
            Self::InMemory(ref store) => store.write(scope, account, token, interactive),
        }
    }

    /// Removes the user token of `account`. Removing a missing token succeeds.
    pub(crate) fn delete(&self, account: &str) -> Result<(), Cause> {
        match *self {
            #[cfg(target_os = "macos")]
            Self::Keychain(ref store) => store.delete(account),
            #[cfg(any(test, feature = "in-memory-store"))]
            Self::InMemory(ref store) => store.delete(account),
        }
    }

    /// Returns whether `no_keyring`, the value of `GGSHIELD_NO_KEYRING`,
    /// disables the OS store. Every value except `false` and `0`, in any case,
    /// disables it. A value that is not valid UTF-8 does not.
    #[cfg_attr(not(target_os = "macos"), allow(dead_code, reason = "no OS store on this platform"))]
    fn disabled(no_keyring: Option<&OsStr>) -> bool {
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
    /// THEN only `false` and `0`, in any case, keep the OS store enabled
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
        assert_eq!(OsStore::disabled(value.map(OsStr::new)), disabled);
    }

    /// GIVEN a value of `GGSHIELD_NO_KEYRING` that is not valid UTF-8
    /// WHEN it is read
    /// THEN the OS store stays enabled
    #[cfg(unix)]
    #[test]
    fn no_keyring_switch_ignores_non_utf8() {
        use std::os::unix::ffi::OsStrExt;

        assert!(!OsStore::disabled(Some(OsStr::from_bytes(b"\xff"))));
    }
}
