use std::{io, ptr, slice};

use windows_sys::Win32::Foundation::ERROR_NOT_FOUND;
use windows_sys::Win32::Security::Credentials::{
    CRED_PERSIST_ENTERPRISE, CRED_TYPE_GENERIC, CREDENTIALW, CredDeleteW, CredFree, CredReadW,
    CredWriteW,
};

use super::{SERVICE, Scope};
use crate::{Cause, OsStoreErrorReason};

pub(super) use CredentialManagerStore as PlatformStore;

const KEYRING_COMMENT: &str = "Stored using python-keyring";
const NOT_FOUND: i32 = ERROR_NOT_FOUND.cast_signed();
const NOT_UTF16: &str = "the stored secret is not UTF-16";
const NO_MACHINE_TOKENS: &str = "the Credential Manager does not hold machine tokens";

/// The Credential Manager of the current user, for user tokens.
#[derive(Debug)]
pub(crate) struct CredentialManagerStore;

impl CredentialManagerStore {
    pub(crate) fn current() -> Option<Self> {
        Some(Self)
    }

    /// Returns the reason a token of `scope` cannot be stored, if any.
    pub(crate) fn check_available(&self, scope: Scope) -> Result<(), OsStoreErrorReason> {
        if scope == Scope::Machine {
            return Err(OsStoreErrorReason::Unavailable(NO_MACHINE_TOKENS.to_owned()));
        }
        Credential::read(SERVICE)
            .map(drop)
            .map_err(|cause| OsStoreErrorReason::Unavailable(cause.to_string()))
    }

    /// Returns the token of `scope` stored for `account`, or `None` if there is
    /// none.
    pub(crate) fn read(&self, scope: Scope, account: &str) -> Result<Option<String>, Cause> {
        if scope == Scope::Machine {
            return Err(Cause::OsStoreUnavailable(NO_MACHINE_TOKENS.to_owned()));
        }
        if let Some(credential) =
            Credential::read(SERVICE)?.filter(|credential| credential.user_name == account)
        {
            return credential.secret().map(Some);
        }
        Credential::read(&compound_target(account))?
            .map(|credential| credential.secret())
            .transpose()
    }

    /// Stores `token` of `scope` for `account`.
    pub(crate) fn write(
        &self,
        scope: Scope,
        account: &str,
        token: &str,
        _interactive: bool,
    ) -> Result<(), Cause> {
        if scope == Scope::Machine {
            return Err(Cause::OsStoreUnavailable(NO_MACHINE_TOKENS.to_owned()));
        }
        if let Some(previous) =
            Credential::read(SERVICE)?.filter(|credential| credential.user_name != account)
        {
            previous.write(&compound_target(&previous.user_name))?;
        }
        Credential::new(account, token).write(SERVICE)
    }

    /// Removes the user token of `account`. Removing a missing token succeeds.
    pub(crate) fn delete(&self, account: &str) -> Result<(), Cause> {
        for target in [SERVICE.to_owned(), compound_target(account)] {
            if Credential::read(&target)?.is_some_and(|credential| credential.user_name == account)
            {
                Credential::delete(&target)?;
            }
        }
        Ok(())
    }
}

fn compound_target(account: &str) -> String {
    format!("{account}@{SERVICE}")
}

struct Credential {
    user_name: String,
    blob: Vec<u8>,
}

impl Credential {
    fn new(user_name: &str, secret: &str) -> Self {
        Self {
            user_name: user_name.to_owned(),
            blob: secret.encode_utf16().flat_map(u16::to_le_bytes).collect(),
        }
    }

    fn read(target: &str) -> Result<Option<Self>, Cause> {
        let target = utf16_with_nul(target);
        let mut buffer = ptr::null_mut();
        // SAFETY: `target` is NUL-terminated and outlives the call, and `buffer`
        // is a valid out-pointer.
        if unsafe { CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut buffer) } == 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(NOT_FOUND) {
                return Ok(None);
            }
            return Err(Cause::OsStoreUnavailable(error.to_string()));
        }
        // SAFETY: `CredReadW` succeeded, so `buffer` points to a credential
        // whose blob has `CredentialBlobSize` bytes, and whose user name is null
        // or NUL-terminated. The buffer is freed once, after the copies.
        let credential = unsafe {
            let fields = &*buffer;
            let blob = if fields.CredentialBlob.is_null() {
                Vec::new()
            } else {
                slice::from_raw_parts(fields.CredentialBlob, fields.CredentialBlobSize as usize)
                    .to_vec()
            };
            let user_name = if fields.UserName.is_null() {
                String::new()
            } else {
                let mut length = 0;
                while *fields.UserName.add(length) != 0 {
                    length += 1;
                }
                String::from_utf16_lossy(slice::from_raw_parts(fields.UserName, length))
            };
            CredFree(buffer.cast_const().cast());
            Self { user_name, blob }
        };
        Ok(Some(credential))
    }

    fn write(&self, target: &str) -> Result<(), Cause> {
        let mut target = utf16_with_nul(target);
        let mut user_name = utf16_with_nul(&self.user_name);
        let mut comment = utf16_with_nul(KEYRING_COMMENT);
        let blob_size = u32::try_from(self.blob.len())
            .map_err(|_| Cause::OsStoreUnavailable("the secret is too long".to_owned()))?;
        let credential = CREDENTIALW {
            Type: CRED_TYPE_GENERIC,
            TargetName: target.as_mut_ptr(),
            Comment: comment.as_mut_ptr(),
            CredentialBlobSize: blob_size,
            CredentialBlob: self.blob.as_ptr().cast_mut(),
            Persist: CRED_PERSIST_ENTERPRISE,
            UserName: user_name.as_mut_ptr(),
            ..CREDENTIALW::default()
        };
        // SAFETY: the strings and the blob outlive the call, and `CredWriteW` only
        // reads them.
        if unsafe { CredWriteW(&credential, 0) } == 0 {
            return Err(Cause::OsStoreUnavailable(io::Error::last_os_error().to_string()));
        }
        Ok(())
    }

    /// Removes the credential of `target`. Removing a missing credential
    /// succeeds.
    fn delete(target: &str) -> Result<(), Cause> {
        let target = utf16_with_nul(target);
        // SAFETY: `target` is NUL-terminated and outlives the call.
        if unsafe { CredDeleteW(target.as_ptr(), CRED_TYPE_GENERIC, 0) } != 0 {
            return Ok(());
        }
        let error = io::Error::last_os_error();
        if error.raw_os_error() == Some(NOT_FOUND) {
            return Ok(());
        }
        Err(Cause::OsStoreUnavailable(error.to_string()))
    }

    fn secret(&self) -> Result<String, Cause> {
        let (bytes, big_endian) = match self.blob.as_slice() {
            [0xff, 0xfe, rest @ ..] => (rest, false),
            [0xfe, 0xff, rest @ ..] => (rest, true),
            bytes => (bytes, false),
        };
        let (pairs, remainder) = bytes.as_chunks::<2>();
        if !remainder.is_empty() {
            return Err(Cause::OsStoreUnavailable(NOT_UTF16.to_owned()));
        }
        let decode = if big_endian { u16::from_be_bytes } else { u16::from_le_bytes };
        let units: Vec<u16> = pairs.iter().copied().map(decode).collect();
        String::from_utf16(&units).map_err(|_| Cause::OsStoreUnavailable(NOT_UTF16.to_owned()))
    }
}

fn utf16_with_nul(value: &str) -> Vec<u16> {
    value.encode_utf16().chain([0]).collect()
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use yare::parameterized;

    use super::*;

    const STORE: CredentialManagerStore = CredentialManagerStore;

    static LIVE: Mutex<()> = Mutex::new(());

    fn require_live() {
        assert!(
            std::env::var_os("GG_CREDENTIAL_STORE_LIVE").is_some(),
            "set GG_CREDENTIAL_STORE_LIVE=1 to run the tests that use the Credential Manager"
        );
    }

    fn account(name: &str) -> String {
        format!("https://{name}-{}.test-client.invalid", std::process::id())
    }

    fn credential(blob: &[u8]) -> Credential {
        Credential {
            user_name: "https://dashboard.gitguardian.com".to_owned(),
            blob: blob.to_vec(),
        }
    }

    /// GIVEN an account
    /// WHEN its compound target is built
    /// THEN it is the account, `@` and the service, as Python `keyring` builds it
    #[test]
    fn compound_target_matches_keyring() {
        assert_eq!(
            compound_target("https://dashboard.gitguardian.com"),
            "https://dashboard.gitguardian.com@gg_credential_store.test"
        );
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

    /// GIVEN the Credential Manager of a logged-on user
    /// WHEN it is checked
    /// THEN it is available
    #[test]
    #[ignore = "uses the Credential Manager; run with GG_CREDENTIAL_STORE_LIVE=1"]
    fn is_available() {
        require_live();

        assert_eq!(STORE.check_available(Scope::User), Ok(()));
    }

    /// GIVEN the Credential Manager
    /// WHEN a token is written, overwritten, read, deleted and read again
    /// THEN the first read returns the second token and the last finds nothing
    #[test]
    #[ignore = "uses the Credential Manager; run with GG_CREDENTIAL_STORE_LIVE=1"]
    fn round_trip() {
        require_live();
        let _live = LIVE.lock().unwrap();
        let account = account("round-trip");

        STORE.write(Scope::User, &account, "first", false).unwrap();
        STORE.write(Scope::User, &account, "second", false).unwrap();
        let read = STORE.read(Scope::User, &account);
        STORE.delete(&account).unwrap();

        assert_eq!(read.unwrap().as_deref(), Some("second"));
        assert_eq!(STORE.read(Scope::User, &account).unwrap(), None);
    }

    /// GIVEN the token of one instance under the plain target
    /// WHEN the token of a second instance is written
    /// THEN the first moves to its compound target, the second is stored under the plain
    /// target, both read back, and deleting both leaves neither target
    #[test]
    #[ignore = "uses the Credential Manager; run with GG_CREDENTIAL_STORE_LIVE=1"]
    fn second_instance_moves_first_to_compound_target() {
        require_live();
        let _live = LIVE.lock().unwrap();
        let first = account("first");
        let second = account("second");

        STORE.write(Scope::User, &first, "first token", false).unwrap();
        STORE.write(Scope::User, &second, "second token", false).unwrap();
        let plain = Credential::read(SERVICE).unwrap().map(|credential| credential.user_name);
        let compound = Credential::read(&compound_target(&first))
            .unwrap()
            .map(|credential| credential.user_name);
        let first_read = STORE.read(Scope::User, &first);
        let second_read = STORE.read(Scope::User, &second);
        STORE.delete(&first).unwrap();
        STORE.delete(&second).unwrap();

        assert_eq!(plain.as_ref(), Some(&second));
        assert_eq!(compound.as_ref(), Some(&first));
        assert_eq!(first_read.unwrap().as_deref(), Some("first token"));
        assert_eq!(second_read.unwrap().as_deref(), Some("second token"));
        assert!(Credential::read(&compound_target(&first)).unwrap().is_none());
        assert!(Credential::read(SERVICE).unwrap().is_none());
    }

    /// GIVEN the Credential Manager without a token for an account
    /// WHEN the token is read and deleted
    /// THEN nothing is found and the delete succeeds
    #[test]
    #[ignore = "uses the Credential Manager; run with GG_CREDENTIAL_STORE_LIVE=1"]
    fn missing_token() {
        require_live();
        let _live = LIVE.lock().unwrap();
        let account = account("missing");

        assert_eq!(STORE.read(Scope::User, &account).unwrap(), None);
        assert!(STORE.delete(&account).is_ok());
    }

    /// GIVEN a secret
    /// WHEN a credential is built from it
    /// THEN the blob holds the secret in UTF-16LE without a byte order mark, as
    /// Python `keyring` writes it
    #[test]
    fn encodes_utf16le() {
        let credential = Credential::new("https://dashboard.gitguardian.com", "gg_é");

        assert_eq!(credential.blob, [b'g', 0, b'g', 0, b'_', 0, 0xe9, 0]);
    }

    /// GIVEN a blob in UTF-16
    /// WHEN its secret is decoded
    /// THEN a byte order mark selects the byte order, and the default is little
    /// endian
    #[parameterized(
        little_endian = { &[b'g', 0, b'g', 0] },
        little_endian_mark = { &[0xFF, 0xFE, b'g', 0, b'g', 0] },
        big_endian_mark = { &[0xFE, 0xFF, 0, b'g', 0, b'g'] },
    )]
    fn decodes_utf16(blob: &[u8]) {
        assert_eq!(credential(blob).secret().unwrap(), "gg");
    }

    /// GIVEN a blob that is not UTF-16
    /// WHEN its secret is decoded
    /// THEN the store is unavailable
    #[parameterized(
        odd_length = { &[b'g', 0, b'g'] },
        unpaired_surrogate = { &[0x00, 0xD8, b'g', 0] },
    )]
    fn rejects_invalid_utf16(blob: &[u8]) {
        assert!(matches!(credential(blob).secret(), Err(Cause::OsStoreUnavailable(_))));
    }
}
