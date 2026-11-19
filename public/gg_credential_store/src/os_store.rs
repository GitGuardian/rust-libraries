#[cfg_attr(target_os = "linux", path = "os_store/linux.rs")]
#[cfg_attr(target_os = "macos", path = "os_store/macos/mod.rs")]
#[cfg_attr(windows, path = "os_store/windows.rs")]
#[cfg_attr(
    not(any(target_os = "linux", target_os = "macos", windows)),
    path = "os_store/unsupported.rs"
)]
mod imp;
#[cfg(any(test, feature = "test-support"))]
mod in_memory;

#[cfg(any(test, feature = "test-support"))]
pub use in_memory::{InMemoryOsStore, OsStoreFailure};

use imp::PlatformStore;

use crate::{Cause, CleartextReason};

#[cfg(not(test))]
#[cfg_attr(not(any(target_os = "linux", target_os = "macos", windows)), expect(dead_code))]
const SERVICE: &str = "ggshield";
#[cfg(test)]
#[cfg_attr(not(any(target_os = "linux", target_os = "macos", windows)), expect(dead_code))]
const SERVICE: &str = "gg_credential_store.test";

/// Whose token an operation concerns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Scope {
    User,
    Machine,
}

/// The credential store of the operating system.
#[derive(Debug)]
pub(crate) enum OsStore {
    Platform(PlatformStore),
    #[cfg(any(test, feature = "test-support"))]
    InMemory(InMemoryOsStore),
}

impl OsStore {
    /// Returns the OS store of this platform, or `None` if it has none.
    pub(crate) fn current() -> Option<Self> {
        PlatformStore::current().map(Self::Platform)
    }

    /// Returns the reason a token of `scope` cannot be stored in this store, if
    /// any.
    pub(crate) fn check_available(&self, scope: Scope) -> Result<(), CleartextReason> {
        match *self {
            Self::Platform(ref store) => store.check_available(scope),
            #[cfg(any(test, feature = "test-support"))]
            Self::InMemory(ref store) => store.check_available(scope),
        }
    }

    /// Returns the token of `scope` stored for `account`, or `None` if there is
    /// none.
    pub(crate) fn read(&self, scope: Scope, account: &str) -> Result<Option<String>, Cause> {
        match *self {
            Self::Platform(ref store) => store.read(scope, account),
            #[cfg(any(test, feature = "test-support"))]
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
            Self::Platform(ref store) => store.write(scope, account, token, interactive),
            #[cfg(any(test, feature = "test-support"))]
            Self::InMemory(ref store) => store.write(scope, account, token, interactive),
        }
    }

    /// Removes the user token of `account`. Removing a missing token succeeds.
    pub(crate) fn delete(&self, account: &str) -> Result<(), Cause> {
        match *self {
            Self::Platform(ref store) => store.delete(account),
            #[cfg(any(test, feature = "test-support"))]
            Self::InMemory(ref store) => store.delete(account),
        }
    }
}
