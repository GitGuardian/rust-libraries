use core_foundation::array::CFArray;
use core_foundation::base::{CFType, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::data::CFData;
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::CFString;

use super::security_sys::{self, Constant, SecAccess, SecKeychain};
use crate::Cause;

#[cfg(not(test))]
pub(super) const SERVICE: &str = "ggshield";
#[cfg(test)]
pub(super) const SERVICE: &str = "gg_credential_store.test";

impl SecKeychain {
    /// Returns the secret of `account`, or `None` if there is none.
    pub(super) fn read(&self, account: &str) -> Result<Option<String>, Cause> {
        let mut query = self.query(account);
        query.push((Constant::ReturnData.cf_string(), CFBoolean::true_value().into_CFType()));
        query.push((
            Constant::MatchLimit.cf_string(),
            Constant::MatchLimitOne.cf_string().into_CFType(),
        ));
        let result = match security_sys::copy_matching(&CFDictionary::from_CFType_pairs(&query)) {
            Ok(result) => result,
            Err(Cause::OsStoreItemMissing) => return Ok(None),
            Err(cause) => return Err(cause),
        };
        let data = result.downcast_into::<CFData>().ok_or_else(|| {
            Cause::OsStoreUnavailable("SecItemCopyMatching returned no data".to_owned())
        })?;
        String::from_utf8(data.bytes().to_vec())
            .map(Some)
            .map_err(|_| Cause::OsStoreUnavailable("the stored secret is not UTF-8".to_owned()))
    }

    /// Replaces the secret of `account` with `secret`, readable by `access`.
    pub(super) fn write(
        &self,
        account: &str,
        secret: &str,
        access: &SecAccess,
    ) -> Result<(), Cause> {
        self.delete(account)?;
        self.add(account, secret, access)
    }

    /// Removes the secret of `account`. Removing a missing secret succeeds.
    pub(super) fn delete(&self, account: &str) -> Result<(), Cause> {
        match security_sys::delete_items(&CFDictionary::from_CFType_pairs(&self.query(account))) {
            Err(Cause::OsStoreItemMissing) => Ok(()),
            result => result,
        }
    }

    fn add(&self, account: &str, secret: &str, access: &SecAccess) -> Result<(), Cause> {
        let attributes = CFDictionary::from_CFType_pairs(&[
            (Constant::Class.cf_string(), Constant::ClassGenericPassword.cf_string().into_CFType()),
            (Constant::AttrService.cf_string(), CFString::new(SERVICE).into_CFType()),
            (Constant::AttrAccount.cf_string(), CFString::new(account).into_CFType()),
            (Constant::ValueData.cf_string(), CFData::from_buffer(secret.as_bytes()).into_CFType()),
            (Constant::UseKeychain.cf_string(), self.as_CFType()),
            (Constant::AttrAccess.cf_string(), access.as_CFType()),
        ]);
        security_sys::add_item(&attributes)
    }

    /// Returns the attributes that select the item of `account` in this
    /// keychain only.
    fn query(&self, account: &str) -> Vec<(CFString, CFType)> {
        let search_list = CFArray::from_CFTypes(std::slice::from_ref(self));
        vec![
            (Constant::Class.cf_string(), Constant::ClassGenericPassword.cf_string().into_CFType()),
            (Constant::AttrService.cf_string(), CFString::new(SERVICE).into_CFType()),
            (Constant::AttrAccount.cf_string(), CFString::new(account).into_CFType()),
            (Constant::MatchSearchList.cf_string(), search_list.into_CFType()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    use std::process::Command;
    use std::time::{Duration, Instant};

    use super::*;
    use crate::os_store::macos::keychain_dialogs::KeychainDialogs;

    fn live() -> (KeychainDialogs, SecKeychain, SecAccess) {
        assert!(
            std::env::var_os("GG_CREDENTIAL_STORE_LIVE").is_some(),
            "set GG_CREDENTIAL_STORE_LIVE=1 to run the tests that use the login keychain"
        );
        let dialogs = KeychainDialogs::forbid().unwrap();
        let executable = std::fs::canonicalize(std::env::current_exe().unwrap()).unwrap();
        let executable = CString::new(executable.as_os_str().as_bytes()).unwrap();
        let access = SecAccess::new(SERVICE, &[&executable]).unwrap();
        (dialogs, SecKeychain::login().unwrap(), access)
    }

    fn account(name: &str) -> String {
        format!("{name}-{}", std::process::id())
    }

    /// GIVEN the login keychain of a desktop session
    /// WHEN its lock state is read
    /// THEN it is unlocked
    #[test]
    #[ignore = "uses the login keychain; run with GG_CREDENTIAL_STORE_LIVE=1"]
    fn login_keychain_is_unlocked() {
        let (_dialogs, keychain, _access) = live();

        assert!(keychain.is_unlocked().unwrap());
    }

    /// GIVEN a stored secret
    /// WHEN another secret is written for the same account
    /// THEN the new secret is read
    #[test]
    #[ignore = "uses the login keychain; run with GG_CREDENTIAL_STORE_LIVE=1"]
    fn write_replaces_secret() {
        let (_dialogs, keychain, access) = live();
        let account = account("replace");

        keychain.write(&account, "first", &access).unwrap();
        keychain.write(&account, "second", &access).unwrap();
        let read = keychain.read(&account);
        keychain.delete(&account).unwrap();

        assert_eq!(read.unwrap().as_deref(), Some("second"));
    }

    /// GIVEN no secret for an account
    /// WHEN it is read and deleted
    /// THEN the read finds nothing and the delete succeeds
    #[test]
    #[ignore = "uses the login keychain; run with GG_CREDENTIAL_STORE_LIVE=1"]
    fn missing_secret_is_absent() {
        let (_dialogs, keychain, _access) = live();
        let account = account("missing");

        assert_eq!(keychain.read(&account).unwrap(), None);
        assert!(keychain.delete(&account).is_ok());
    }

    /// GIVEN an item that trusts only `/usr/bin/true`
    /// WHEN the test binary reads it while dialogs are forbidden
    /// THEN the read fails at once, without a dialog
    #[test]
    #[ignore = "uses the login keychain; run with GG_CREDENTIAL_STORE_LIVE=1"]
    fn forbidden_dialog_denies_untrusted_read() {
        let (_dialogs, keychain, _access) = live();
        let account = account("untrusted");
        let foreign = SecAccess::new(SERVICE, &[c"/usr/bin/true"]).unwrap();
        keychain.write(&account, "secret", &foreign).unwrap();

        let started = Instant::now();
        let read = keychain.read(&account);
        let elapsed = started.elapsed();
        let deleted = Command::new("/usr/bin/security")
            .args(["delete-generic-password", "-s", SERVICE, "-a", &account])
            .output()
            .unwrap();

        assert!(matches!(read, Err(Cause::OsStoreDenied)), "{read:?}");
        assert!(elapsed < Duration::from_secs(1), "{elapsed:?}");
        assert!(deleted.status.success(), "{}", String::from_utf8_lossy(&deleted.stderr));
    }
}
