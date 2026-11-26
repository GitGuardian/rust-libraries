use std::ffi::{CStr, CString};
use std::path::PathBuf;
use std::time::Duration;

use gg_fs::FileLock;
use gg_fs::locations::{Dir, Locations};

use super::caller::Caller;
use super::keychain_dialogs::KeychainDialogs;
use super::launcher::Launcher;
use super::security_sys::{SecAccess, SecKeychain};
use crate::os_store::{SERVICE, Scope};
use crate::{Cause, OsStoreErrorReason};

const LOCK_FILE_NAME: &str = "keyring.lock";
const LOCK_TIMEOUT: Duration = Duration::from_secs(10);

/// The login keychain for user tokens and the System keychain for machine
/// tokens.
#[derive(Debug)]
pub(crate) struct KeychainStore {
    caller: Caller,
    lock: Option<PathBuf>,
}

impl KeychainStore {
    pub(crate) fn current() -> Option<Self> {
        let lock = Locations::current()
            .dir(Dir::Cache)
            .inspect_err(|err| tracing::warn!("keychain calls proceed without a lock: {err}"))
            .ok()
            .map(|dir| dir.join(LOCK_FILE_NAME));
        Some(Self { caller: Caller::current(), lock })
    }

    /// Returns the reason a token of `scope` cannot be stored, if any.
    pub(crate) fn check_available(&self, scope: Scope) -> Result<(), OsStoreErrorReason> {
        match (scope, self.caller) {
            (_, Caller::Unsigned) => Err(OsStoreErrorReason::UnsupportedCaller),
            (Scope::Machine, Caller::Signed) => Ok(()),
            (Scope::User, Caller::Signed) => SecKeychain::login().map(drop).map_err(|cause| {
                OsStoreErrorReason::Unavailable(format!(
                    "the login keychain cannot be opened: {cause}"
                ))
            }),
        }
    }

    /// Returns the token of `scope` stored for `account`, or `None` if there is
    /// none.
    pub(crate) fn read(&self, scope: Scope, account: &str) -> Result<Option<String>, Cause> {
        if scope == Scope::Machine && self.caller == Caller::Unsigned {
            return Err(Cause::OsStoreDenied);
        }
        let _dialogs = KeychainDialogs::forbid()?;
        if scope == Scope::Machine {
            return SecKeychain::system()?.read(account);
        }
        let _lock = self.lock();
        match self.caller {
            Caller::Signed => {
                let keychain = SecKeychain::login()?;
                if !keychain.is_unlocked()? {
                    return Err(Cause::OsStoreLocked("the login keychain is locked".to_owned()));
                }
                keychain.read(account)
            }
            Caller::Unsigned => {
                SecKeychain::login().and_then(|keychain| keychain.read(account)).map_err(|cause| {
                    tracing::debug!("{account}: the silent read failed: {cause}");
                    Cause::OsStoreDenied
                })
            }
        }
    }

    /// Stores `token` of `scope` for `account`. With `interactive`, the
    /// keychain may ask the user to unlock it.
    pub(crate) fn write(
        &self,
        scope: Scope,
        account: &str,
        token: &str,
        interactive: bool,
    ) -> Result<(), Cause> {
        if self.caller == Caller::Unsigned {
            return Err(Cause::OsStoreDenied);
        }
        let paths = Launcher::trusted_paths()?;
        let paths: Vec<&CStr> = paths.iter().map(CString::as_c_str).collect();
        let access = SecAccess::new(SERVICE, &paths)?;
        let _dialogs =
            if interactive { KeychainDialogs::allow()? } else { KeychainDialogs::forbid()? };
        if scope == Scope::Machine {
            return SecKeychain::system()?.write(account, token, &access);
        }
        let _lock = self.lock();
        let keychain = SecKeychain::login()?;
        if !interactive && !keychain.is_unlocked()? {
            return Err(Cause::OsStoreLocked("the login keychain is locked".to_owned()));
        }
        keychain.write(account, token, &access)
    }

    /// Removes the user token of `account`. Removing a missing token succeeds.
    pub(crate) fn delete(&self, account: &str) -> Result<(), Cause> {
        if self.caller == Caller::Unsigned {
            return Err(Cause::OsStoreDenied);
        }
        let _dialogs = KeychainDialogs::forbid()?;
        let _lock = self.lock();
        let keychain = SecKeychain::login()?;
        if !keychain.is_unlocked()? {
            return Err(Cause::OsStoreLocked("the login keychain is locked".to_owned()));
        }
        keychain.delete(account)
    }

    /// Returns the keychain lock shared with other GitGuardian programs, or
    /// `None` if it cannot be taken.
    fn lock(&self) -> Option<FileLock> {
        let path = self.lock.as_ref()?;
        FileLock::lock(path, LOCK_TIMEOUT)
            .inspect_err(|err| tracing::warn!("the keychain call proceeds without the lock: {err}"))
            .ok()
    }
}

#[cfg(test)]
mod tests {
    use gg_fs::temp::TempDir;
    use yare::parameterized;

    use super::*;

    const SIGNED: KeychainStore = KeychainStore { caller: Caller::Signed, lock: None };
    const UNSIGNED: KeychainStore = KeychainStore { caller: Caller::Unsigned, lock: None };

    fn require_live() {
        assert!(
            std::env::var_os("GG_CREDENTIAL_STORE_LIVE").is_some(),
            "set GG_CREDENTIAL_STORE_LIVE=1 to run the tests that use the login keychain"
        );
    }

    fn account(name: &str) -> String {
        format!("store-{name}-{}", std::process::id())
    }

    /// GIVEN a program that GitGuardian did not sign
    /// WHEN it checks, writes and deletes a user token
    /// THEN every operation is refused without access to the keychain
    #[test]
    fn unsigned_caller_is_refused() {
        assert_eq!(
            UNSIGNED.check_available(Scope::User),
            Err(OsStoreErrorReason::UnsupportedCaller)
        );
        assert!(matches!(
            UNSIGNED.write(Scope::User, "account", "token", false),
            Err(Cause::OsStoreDenied)
        ));
        assert!(matches!(UNSIGNED.delete("account"), Err(Cause::OsStoreDenied)));
    }

    /// GIVEN a program that GitGuardian did not sign
    /// WHEN it checks, reads and writes a machine token
    /// THEN the read is denied and nothing can be stored
    #[test]
    fn unsigned_caller_has_no_machine_token() {
        assert_eq!(
            UNSIGNED.check_available(Scope::Machine),
            Err(OsStoreErrorReason::UnsupportedCaller)
        );
        assert!(matches!(UNSIGNED.read(Scope::Machine, "account"), Err(Cause::OsStoreDenied)));
        assert!(matches!(
            UNSIGNED.write(Scope::Machine, "account", "token", false),
            Err(Cause::OsStoreDenied)
        ));
    }

    /// GIVEN a signed caller and no machine token for an account
    /// WHEN it reads the machine token of that account
    /// THEN nothing is found
    #[test]
    #[ignore = "uses the System keychain; run with GG_CREDENTIAL_STORE_LIVE=1"]
    fn missing_machine_token_is_absent() {
        require_live();

        assert_eq!(SIGNED.read(Scope::Machine, &account("missing-machine")).unwrap(), None);
    }

    /// GIVEN a signed caller
    /// WHEN it writes a token, reads it, deletes it and reads again
    /// THEN the first read returns it and the second finds nothing
    #[test]
    #[ignore = "uses the login keychain; run with GG_CREDENTIAL_STORE_LIVE=1"]
    fn signed_round_trip() {
        require_live();
        let account = account("round-trip");

        SIGNED.write(Scope::User, &account, "token", false).unwrap();
        let read = SIGNED.read(Scope::User, &account);
        SIGNED.delete(&account).unwrap();

        assert_eq!(read.unwrap().as_deref(), Some("token"));
        assert_eq!(SIGNED.read(Scope::User, &account).unwrap(), None);
    }

    /// GIVEN a token in an item that trusts the test binary
    /// WHEN a program that GitGuardian did not sign reads it
    /// THEN the token is returned
    #[test]
    #[ignore = "uses the login keychain; run with GG_CREDENTIAL_STORE_LIVE=1"]
    fn unsigned_silent_read() {
        require_live();
        let account = account("silent-read");
        SIGNED.write(Scope::User, &account, "token", false).unwrap();

        let read = UNSIGNED.read(Scope::User, &account);
        SIGNED.delete(&account).unwrap();

        assert_eq!(read.unwrap().as_deref(), Some("token"));
    }

    /// GIVEN a lock path in a directory that can be created
    /// WHEN the lock is taken
    /// THEN it is held, in a file at that path
    #[test]
    fn lock_is_taken() {
        let directory = TempDir::new("keychain_store").unwrap();
        let path = directory.path().join("cache").join(LOCK_FILE_NAME);
        let store = KeychainStore { caller: Caller::Signed, lock: Some(path.clone()) };

        let lock = store.lock();

        assert!(lock.is_some());
        assert!(path.is_file());
    }

    /// GIVEN no lock path, or one below a regular file
    /// WHEN the lock is taken
    /// THEN there is no lock
    #[parameterized(
        no_path = { false },
        below_file = { true },
    )]
    fn unusable_lock_is_absent(below_file: bool) {
        let directory = TempDir::new("keychain_store").unwrap();
        let file = directory.path().join("file");
        std::fs::write(&file, "").unwrap();
        let lock = below_file.then(|| file.join(LOCK_FILE_NAME));
        let store = KeychainStore { caller: Caller::Signed, lock };

        assert!(store.lock().is_none());
    }
}
