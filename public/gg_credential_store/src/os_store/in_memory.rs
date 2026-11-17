use std::collections::HashMap;
use std::fmt;
use std::sync::{Mutex, MutexGuard, PoisonError};

use super::Scope;
use crate::{Cause, CleartextReason};

/// An OS store that keeps tokens in memory, for tests.
#[derive(Default)]
pub struct InMemoryOsStore {
    items: Mutex<HashMap<(Scope, String), String>>,
    failure: Option<OsStoreFailure>,
}

/// A failure of an OS store, as [`InMemoryOsStore::failing`] reproduces it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OsStoreFailure {
    /// The store does not serve this caller. Every operation fails.
    UnsupportedCaller,
    /// The store cannot be reached. Every operation fails.
    Unavailable,
    /// The store is locked. Every operation fails.
    Locked,
    /// The store refuses to write tokens. Reading and removing tokens succeed.
    WriteFailed,
}

impl InMemoryOsStore {
    /// Returns an empty store whose operations fail with `failure`.
    pub fn failing(failure: OsStoreFailure) -> Self {
        Self { failure: Some(failure), ..Self::default() }
    }

    fn items(&self) -> MutexGuard<'_, HashMap<(Scope, String), String>> {
        self.items.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(super) fn check_available(&self, _scope: Scope) -> Result<(), CleartextReason> {
        match self.failure {
            Some(OsStoreFailure::UnsupportedCaller) => Err(CleartextReason::UnsupportedCaller),
            Some(OsStoreFailure::Unavailable) => Err(CleartextReason::Unavailable),
            Some(OsStoreFailure::Locked | OsStoreFailure::WriteFailed) | None => Ok(()),
        }
    }

    fn check_access(&self) -> Result<(), Cause> {
        match self.failure {
            Some(OsStoreFailure::UnsupportedCaller) => Err(Cause::OsStoreDenied),
            Some(OsStoreFailure::Unavailable) => {
                Err(Cause::OsStoreUnavailable("no store is reachable".to_owned()))
            }
            Some(OsStoreFailure::Locked) => {
                Err(Cause::OsStoreLocked("the store is locked".to_owned()))
            }
            Some(OsStoreFailure::WriteFailed) | None => Ok(()),
        }
    }

    pub(super) fn read(&self, scope: Scope, account: &str) -> Result<Option<String>, Cause> {
        self.check_access()?;
        Ok(self.items().get(&(scope, account.to_owned())).cloned())
    }

    pub(super) fn write(
        &self,
        scope: Scope,
        account: &str,
        token: &str,
        _interactive: bool,
    ) -> Result<(), Cause> {
        self.check_access()?;
        if self.failure == Some(OsStoreFailure::WriteFailed) {
            return Err(Cause::OsStoreUnavailable("the store refused the token".to_owned()));
        }
        self.items().insert((scope, account.to_owned()), token.to_owned());
        Ok(())
    }

    pub(super) fn delete(&self, account: &str) -> Result<(), Cause> {
        self.check_access()?;
        self.items().remove(&(Scope::User, account.to_owned()));
        Ok(())
    }
}

impl fmt::Debug for InMemoryOsStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InMemoryOsStore").field("failure", &self.failure).finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACCOUNT: &str = "https://dashboard.gitguardian.com";

    /// GIVEN a user token and a machine token for the same account
    /// WHEN each scope is read, and the account is deleted
    /// THEN each scope returns its own token, and the delete keeps the machine
    /// token
    #[test]
    fn scopes_are_separate() {
        let store = InMemoryOsStore::default();
        store.write(Scope::User, ACCOUNT, "user-token", false).unwrap();
        store.write(Scope::Machine, ACCOUNT, "machine-token", false).unwrap();

        assert_eq!(store.read(Scope::User, ACCOUNT).unwrap().as_deref(), Some("user-token"));
        assert_eq!(store.read(Scope::Machine, ACCOUNT).unwrap().as_deref(), Some("machine-token"));
        store.delete(ACCOUNT).unwrap();
        assert_eq!(store.read(Scope::User, ACCOUNT).unwrap(), None);
        assert_eq!(store.read(Scope::Machine, ACCOUNT).unwrap().as_deref(), Some("machine-token"));
    }

    /// GIVEN an in-memory store without an item
    /// WHEN the item is deleted
    /// THEN the delete succeeds
    #[test]
    fn deletes_missing_item() {
        assert!(InMemoryOsStore::default().delete(ACCOUNT).is_ok());
    }

    /// GIVEN an in-memory store holding a token
    /// WHEN it is formatted with Debug
    /// THEN the token does not appear
    #[test]
    fn debug_hides_tokens() {
        let store = InMemoryOsStore::default();
        store.write(Scope::User, ACCOUNT, "secret-token", false).unwrap();

        let debug = format!("{store:?}");

        assert!(!debug.contains("secret-token"), "{debug}");
    }
}
