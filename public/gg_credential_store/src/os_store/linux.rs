use std::collections::HashMap;

use secret_service::blocking::SecretService;
use secret_service::{EncryptionType, Error};

use super::{SERVICE, Scope};
use crate::{Cause, OsStoreErrorReason};

pub(super) use SecretServiceStore as PlatformStore;

/// The `application` attribute of the items that Python `keyring` writes.
const KEYRING_APPLICATION: &str = "Python keyring library";
const CONTENT_TYPE: &str = "text/plain";
const CONNECT_ATTEMPTS: u32 = 3;
const INVALID_ARGS: &str = "org.freedesktop.DBus.Error.InvalidArgs";
const NO_MACHINE_TOKENS: &str = "the Secret Service does not hold machine tokens";

/// The Secret Service of the desktop session, for user tokens.
#[derive(Debug)]
pub(crate) struct SecretServiceStore;

impl SecretServiceStore {
    pub(crate) fn current() -> Option<Self> {
        Some(Self)
    }

    /// Returns the reason a token of `scope` cannot be stored, if any.
    pub(crate) fn check_available(&self, scope: Scope) -> Result<(), OsStoreErrorReason> {
        if scope == Scope::Machine {
            return Err(OsStoreErrorReason::Unavailable(NO_MACHINE_TOKENS.to_owned()));
        }
        let service =
            connect().map_err(|cause| OsStoreErrorReason::Unavailable(cause.to_string()))?;
        service.get_default_collection().map(drop).map_err(|err| {
            OsStoreErrorReason::Unavailable(format!(
                "the Secret Service has no default collection: {err}"
            ))
        })
    }

    /// Returns the token of `scope` stored for `account`, or `None` if there is
    /// none.
    pub(crate) fn read(&self, scope: Scope, account: &str) -> Result<Option<String>, Cause> {
        if scope == Scope::Machine {
            return Err(Cause::OsStoreUnavailable(NO_MACHINE_TOKENS.to_owned()));
        }
        let service = connect()?;
        let items = service.search_items(attributes(account)).map_err(cause)?;
        let Some(item) = items.unlocked.first() else {
            if items.locked.is_empty() {
                return Ok(None);
            }
            return Err(Cause::OsStoreLocked("the item is locked".to_owned()));
        };
        let secret = item.get_secret().map_err(cause)?;
        String::from_utf8(secret)
            .map(Some)
            .map_err(|_| Cause::OsStoreUnavailable("the stored secret is not UTF-8".to_owned()))
    }

    /// Stores `token` of `scope` for `account`. With `interactive`, the
    /// Secret Service may ask the user to unlock the default collection.
    pub(crate) fn write(
        &self,
        scope: Scope,
        account: &str,
        token: &str,
        interactive: bool,
    ) -> Result<(), Cause> {
        if scope == Scope::Machine {
            return Err(Cause::OsStoreUnavailable(NO_MACHINE_TOKENS.to_owned()));
        }
        let service = connect()?;
        let collection = service.get_default_collection().map_err(cause)?;
        let locked = collection.is_locked().map_err(cause)?;
        if locked && !interactive {
            return Err(Cause::OsStoreLocked("the default collection is locked".to_owned()));
        }
        if locked {
            collection.unlock().map_err(cause)?;
        }
        let mut attributes = attributes(account);
        attributes.insert("application", KEYRING_APPLICATION);
        let label = format!("Password for '{account}' on '{SERVICE}'");
        collection
            .create_item(&label, attributes, token.as_bytes(), true, CONTENT_TYPE)
            .map(drop)
            .map_err(cause)
    }

    /// Removes the user token of `account`. Removing a missing token succeeds.
    pub(crate) fn delete(&self, account: &str) -> Result<(), Cause> {
        let service = connect()?;
        let items = service.search_items(attributes(account)).map_err(cause)?;
        for item in items.unlocked.iter().chain(&items.locked) {
            item.delete().map_err(cause)?;
        }
        Ok(())
    }
}

fn connect() -> Result<SecretService<'static>, Cause> {
    let mut attempt = 1;
    loop {
        match SecretService::connect(EncryptionType::Dh) {
            Err(err) if attempt < CONNECT_ATTEMPTS && rejects_public_key(&err) => attempt += 1,
            result => return result.map_err(cause),
        }
    }
}

fn rejects_public_key(err: &Error) -> bool {
    matches!(err, Error::Zbus(zbus::Error::MethodError(name, ..)) if name.as_str() == INVALID_ARGS)
}

/// Returns the attributes by which Python `keyring` finds the token of
/// `account`.
fn attributes(account: &str) -> HashMap<&str, &str> {
    HashMap::from([("service", SERVICE), ("username", account)])
}

fn cause(err: Error) -> Cause {
    match err {
        Error::Locked | Error::Prompt => Cause::OsStoreLocked(err.to_string()),
        err => Cause::OsStoreUnavailable(err.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use yare::parameterized;
    use zbus::Message;

    use super::*;

    const STORE: SecretServiceStore = SecretServiceStore;

    fn require_live() {
        assert!(
            std::env::var_os("GG_CREDENTIAL_STORE_LIVE").is_some(),
            "set GG_CREDENTIAL_STORE_LIVE=1 to run the tests that use the Secret Service"
        );
    }

    fn account(name: &str) -> String {
        format!("https://{name}-{}.test-client.invalid", std::process::id())
    }

    fn method_error(name: &str) -> Error {
        let call = Message::method_call("/org/freedesktop/secrets", "OpenSession")
            .unwrap()
            .build(&())
            .unwrap();
        let reply = Message::error(&call.header(), name)
            .unwrap()
            .build(&("Client public key size is invalid",))
            .unwrap();
        Error::Zbus(zbus::Error::from(reply))
    }

    /// GIVEN an account
    /// WHEN its attributes are built
    /// THEN they are the service and the account, as Python `keyring` searches
    /// them
    #[test]
    fn attributes_match_keyring() {
        let expected = HashMap::from([
            ("service", "gg_credential_store.test"),
            ("username", "https://dashboard.gitguardian.com"),
        ]);

        assert_eq!(attributes("https://dashboard.gitguardian.com"), expected);
    }

    /// GIVEN an error of the Secret Service
    /// WHEN it is mapped to a cause
    /// THEN a lock or a dismissed prompt is a locked store, and anything else an
    /// unavailable one
    #[parameterized(
        locked = { Error::Locked, true },
        prompt_dismissed = { Error::Prompt, true },
        prompt_disconnected = { Error::PromptDisconnected, false },
        unavailable = { Error::Unavailable, false },
        no_result = { Error::NoResult, false },
        crypto = { Error::Crypto("bad padding"), false },
    )]
    fn maps_errors(err: Error, locked: bool) {
        let cause = cause(err);

        if locked {
            assert!(matches!(cause, Cause::OsStoreLocked(_)), "{cause}");
        } else {
            assert!(matches!(cause, Cause::OsStoreUnavailable(_)), "{cause}");
        }
    }

    /// GIVEN an error of the Secret Service
    /// WHEN the connection decides whether to open the session again
    /// THEN it does so only for invalid arguments
    #[parameterized(
        invalid_args = { method_error(INVALID_ARGS), true },
        not_supported = { method_error("org.freedesktop.DBus.Error.NotSupported"), false },
        unavailable = { Error::Unavailable, false },
    )]
    fn retries_rejected_public_key(err: Error, expected: bool) {
        assert_eq!(rejects_public_key(&err), expected);
    }

    /// GIVEN the machine scope
    /// WHEN a token is checked, read and written
    /// THEN the store is unavailable for each of them
    #[test]
    fn machine_scope_is_unavailable() {
        assert_eq!(
            STORE.check_available(Scope::Machine),
            Err(OsStoreErrorReason::Unavailable(NO_MACHINE_TOKENS.to_owned()))
        );
        assert!(matches!(STORE.read(Scope::Machine, "account"), Err(Cause::OsStoreUnavailable(_))));
        assert!(matches!(
            STORE.write(Scope::Machine, "account", "token", false),
            Err(Cause::OsStoreUnavailable(_))
        ));
    }

    /// GIVEN an unlocked Secret Service
    /// WHEN it is checked
    /// THEN it is available
    #[test]
    #[ignore = "uses the Secret Service; run with GG_CREDENTIAL_STORE_LIVE=1"]
    fn is_available() {
        require_live();

        assert_eq!(STORE.check_available(Scope::User), Ok(()));
    }

    /// GIVEN an unlocked Secret Service
    /// WHEN a token is written, overwritten, read, deleted and read again
    /// THEN the first read returns the second token and the last finds nothing
    #[test]
    #[ignore = "uses the Secret Service; run with GG_CREDENTIAL_STORE_LIVE=1"]
    fn round_trip() {
        require_live();
        let account = account("round-trip");

        STORE.write(Scope::User, &account, "first", false).unwrap();
        STORE.write(Scope::User, &account, "second", false).unwrap();
        let read = STORE.read(Scope::User, &account);
        STORE.delete(&account).unwrap();

        assert_eq!(read.unwrap().as_deref(), Some("second"));
        assert_eq!(STORE.read(Scope::User, &account).unwrap(), None);
    }

    /// GIVEN an unlocked Secret Service without a token for an account
    /// WHEN the token is read and deleted
    /// THEN nothing is found and the delete succeeds
    #[test]
    #[ignore = "uses the Secret Service; run with GG_CREDENTIAL_STORE_LIVE=1"]
    fn missing_token() {
        require_live();
        let account = account("missing");

        assert_eq!(STORE.read(Scope::User, &account).unwrap(), None);
        assert!(STORE.delete(&account).is_ok());
    }
}
