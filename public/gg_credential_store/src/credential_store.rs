use std::ffi::OsStr;
use std::io;
use std::path::PathBuf;

use chrono::Utc;
use gg_fs::locations::Locations;
use gg_url::DashboardUrl;

use crate::auth_config::{Account, AccountToken, AuthConfig, Instance};
use crate::machine_credential::MachineCredential;
use crate::os_store::{OsStore, Scope};
use crate::{
    Cause, Credential, CredentialStoreError, DeleteOutcome, ListOutcome, OsStoreErrorReason,
    ReadRequest, Setting, SkipReason, Source, StoreOutcome, StoredToken, TokenInfo,
    UnreadableEntry,
};

/// The prefix of a machine token.
pub const MACHINE_TOKEN_PREFIX: &str = "gg_mat_";

const NO_KEYRING_VAR: &str = "GGSHIELD_NO_KEYRING";
const DISABLED: &str = "disabled by GGSHIELD_NO_KEYRING";
const NO_OS_STORE: &str = "this platform has no OS store";
const NOT_IN_OS_STORE: &str = "the OS store holds no machine token for the instance";

/// Reads and writes the GitGuardian tokens of a device.
#[derive(Debug)]
pub struct CredentialStore {
    auth_config: PathBuf,
    /// The path of `machine_credential.yaml`.
    machine_credential: Option<PathBuf>,
    /// The credential store of the operating system, if the platform has one.
    os_store: Option<OsStore>,
    /// Whether `GGSHIELD_NO_KEYRING` keeps user tokens out of the OS store.
    user_os_store_disabled: bool,
}

/// The result of looking up the machine token of an instance.
enum MachineTokenLookup {
    /// The device has no machine token.
    Absent,
    Usable(Credential),
    Skipped(SkipReason),
}

impl CredentialStore {
    /// Returns the credential store of the current user.
    ///
    /// # Errors
    ///
    /// Fails with [`Cause::NoHome`] if no home directory can be determined.
    pub fn new() -> Result<Self, CredentialStoreError> {
        let no_keyring = std::env::var_os(NO_KEYRING_VAR);
        let user_os_store_disabled = no_keyring
            .as_deref()
            .and_then(OsStr::to_str)
            .is_some_and(|value| !matches!(value.to_lowercase().as_str(), "false" | "0"));
        Ok(Self {
            auth_config: AuthConfig::path(&Locations::current())?,
            machine_credential: MachineCredential::path(),
            os_store: OsStore::current(),
            user_os_store_disabled,
        })
    }

    /// Stores the token `token` of `instance`, with its details `info`.
    ///
    /// A machine token, which starts with [`MACHINE_TOKEN_PREFIX`], replaces
    /// the machine token of the device and is recorded with `machine_id`, the
    /// caller's machine id. Any other token is written to the OS store if it
    /// can be used, and to `auth_config.yaml` otherwise. Returns the location
    /// of the token.
    ///
    /// # Errors
    ///
    /// Fails with [`Cause::MissingMachineId`] for a machine token without
    /// `machine_id`, and with [`Cause::MachineStoreDenied`] if the caller may
    /// not write the machine store. Fails if a file cannot be read, is
    /// malformed or cannot be written.
    pub fn store(
        &self,
        instance: &DashboardUrl,
        token: &str,
        info: &TokenInfo,
        machine_id: Option<&str>,
    ) -> Result<StoreOutcome, CredentialStoreError> {
        if token.starts_with(MACHINE_TOKEN_PREFIX) {
            self.store_machine_token(instance, token, info, machine_id)
        } else {
            self.store_user_token(instance, token, info, /* interactive */ false)
        }
    }

    /// Stores a token like [`store`](Self::store), and lets the OS store ask
    /// the user to unlock it.
    ///
    /// # Errors
    ///
    /// Fails with [`Cause::MachineStoreDenied`] for a machine token. Fails if
    /// `auth_config.yaml` cannot be read, is malformed or cannot be written.
    pub fn store_interactive(
        &self,
        instance: &DashboardUrl,
        token: &str,
        info: &TokenInfo,
    ) -> Result<StoreOutcome, CredentialStoreError> {
        self.store_user_token(instance, token, info, /* interactive */ true)
    }

    /// Removes the user token of `instance` from `auth_config.yaml` and from
    /// the OS store.
    ///
    /// # Errors
    ///
    /// Fails if `auth_config.yaml` cannot be read, is malformed or cannot be
    /// written.
    pub fn delete(&self, instance: &DashboardUrl) -> Result<DeleteOutcome, CredentialStoreError> {
        let mut config = AuthConfig::load(&self.auth_config)?;
        let Some(entry) = config.instance_mut(instance) else {
            return Ok(DeleteOutcome::NotStored);
        };
        let [account] = &mut entry.accounts;
        let Some(account) = account.take() else {
            return Ok(DeleteOutcome::NotStored);
        };
        let os_store = self
            .user_os_store()
            .and_then(|os_store| os_store.check_available(Scope::User).map(|()| os_store));
        let removed = match (account.token, os_store) {
            (_, Ok(os_store)) => os_store.delete(&entry.url).map_err(OsStoreErrorReason::refused),
            (AccountToken::InOsStore, Err(reason)) => Err(reason),
            (AccountToken::Cleartext(_), Err(_)) => Ok(()),
        };
        config.save(&self.auth_config)?;
        Ok(match removed {
            Ok(()) => DeleteOutcome::Removed,
            Err(reason) => DeleteOutcome::KeptInOsStore(reason),
        })
    }

    /// Changes `setting` in `auth_config.yaml`.
    ///
    /// # Errors
    ///
    /// Fails if `auth_config.yaml` cannot be read, is malformed or cannot be
    /// written.
    pub fn set_setting(&self, setting: Setting) -> Result<(), CredentialStoreError> {
        let mut config = AuthConfig::load(&self.auth_config)?;
        match setting {
            Setting::DefaultTokenLifetime { instance: None, days } => {
                config.default_token_lifetime = days;
            }
            Setting::DefaultTokenLifetime { instance: Some(instance), days } => {
                match config.instance_mut(&instance) {
                    Some(entry) => entry.default_token_lifetime = days,
                    None if days.is_none() => return Ok(()),
                    None => config.instances.push(Instance {
                        default_token_lifetime: days,
                        ..Instance::new(&instance)
                    }),
                }
            }
        }
        config.save(&self.auth_config)
    }

    /// Returns the token to use for `instance`.
    ///
    /// Returns the machine token if it belongs to `instance`, can be read,
    /// grants the required scopes of `request` and has not expired. If the
    /// device has no machine token, returns the explicit token of `request`, if
    /// any. Otherwise returns the user token.
    ///
    /// # Errors
    ///
    /// Fails with [`Cause::NoToken`] if no token is available, and with
    /// [`Cause::MachineTokenSkipped`] if the machine token was not returned and
    /// no user token is stored. Fails with [`Cause::Expired`] if the user token
    /// has expired. Fails if the user token is in the OS store and the OS store
    /// cannot return it, or if `auth_config.yaml` cannot be read or is
    /// malformed.
    pub fn read(
        &self,
        instance: &DashboardUrl,
        request: &ReadRequest,
    ) -> Result<Credential, CredentialStoreError> {
        if request.skip_machine_token {
            return self.read_user_token(instance, None);
        }
        let skipped = match self.read_machine_token(instance, &request.required_scopes) {
            MachineTokenLookup::Usable(credential) => return Ok(credential),
            MachineTokenLookup::Skipped(reason) => Some(reason),
            MachineTokenLookup::Absent => None,
        };
        if skipped.is_none()
            && let Some(token) = request.explicit_token.as_deref().filter(|token| !token.is_empty())
        {
            return Ok(Credential { token: token.to_owned(), source: Source::Explicit });
        }
        self.read_user_token(instance, skipped)
    }

    /// Returns the stored tokens, without their secrets, and the entries that
    /// cannot be read.
    ///
    /// # Errors
    ///
    /// Fails if `auth_config.yaml` cannot be read or is malformed.
    pub fn list(&self) -> Result<ListOutcome, CredentialStoreError> {
        let config = AuthConfig::load(&self.auth_config)?;
        let mut outcome = ListOutcome { tokens: Vec::new(), unreadable: Vec::new() };
        if let Some(path) = self.machine_credential.as_deref() {
            match MachineCredential::load(path) {
                Ok(Some(record)) => outcome.tokens.push(record.stored_token()),
                Ok(None) => {}
                Err(err) => outcome.unreadable.push(UnreadableEntry::MachineFile {
                    path: path.to_path_buf(),
                    reason: err.cause().to_string(),
                }),
            }
        }
        for entry in &config.instances {
            let [Some(account)] = &entry.accounts else {
                continue;
            };
            match entry.url.parse() {
                Ok(instance) => outcome.tokens.push(StoredToken {
                    instance,
                    instance_name: entry.name.clone(),
                    info: account.info(),
                    location: account.token.location(),
                }),
                Err(err) => outcome.unreadable.push(UnreadableEntry::UserInstance {
                    url: entry.url.clone(),
                    reason: err.to_string(),
                }),
            }
        }
        Ok(outcome)
    }

    /// Returns the instance this device is enrolled with, if any.
    ///
    /// # Errors
    ///
    /// Fails if `machine_credential.yaml` is not trusted, cannot be read or is
    /// malformed, including a malformed instance URL.
    pub fn managed_instance(&self) -> Result<Option<DashboardUrl>, CredentialStoreError> {
        let Some(path) = self.machine_credential.as_deref() else {
            return Ok(None);
        };
        Ok(MachineCredential::load(path)?.map(|record| record.instance))
    }

    /// Returns the lifetime in days of new tokens for `instance`. Without a
    /// setting for `instance`, or with `None`, returns the global setting.
    ///
    /// # Errors
    ///
    /// Fails if `auth_config.yaml` cannot be read or is malformed.
    pub fn default_token_lifetime(
        &self,
        instance: Option<&DashboardUrl>,
    ) -> Result<Option<u32>, CredentialStoreError> {
        let config = AuthConfig::load(&self.auth_config)?;
        let own = instance
            .and_then(|instance| config.instance(instance))
            .and_then(|entry| entry.default_token_lifetime);
        Ok(own.or(config.default_token_lifetime))
    }

    /// Returns the machine token for a read of `instance` that requires
    /// `required_scopes`, or the reason it is skipped.
    fn read_machine_token(
        &self,
        instance: &DashboardUrl,
        required_scopes: &[String],
    ) -> MachineTokenLookup {
        let Some(path) = self.machine_credential.as_deref() else {
            return MachineTokenLookup::Absent;
        };
        let record = match MachineCredential::load(path) {
            Ok(Some(record)) => record,
            Ok(None) => return MachineTokenLookup::Absent,
            Err(err) => {
                return MachineTokenLookup::Skipped(SkipReason::Unreadable(err.to_string()));
            }
        };
        if record.instance != *instance {
            let reason = SkipReason::OtherInstance(record.instance.to_string());
            return MachineTokenLookup::Skipped(reason);
        }
        if let Some(reason) = record.skip_reason(required_scopes) {
            return MachineTokenLookup::Skipped(reason);
        }
        let machine_id = record.machine_id;
        if let Some(token) = record.token {
            return MachineTokenLookup::Usable(Credential {
                token,
                source: Source::MachineFile { machine_id },
            });
        }
        let Some(os_store) = &self.os_store else {
            return MachineTokenLookup::Skipped(SkipReason::Unreadable(NO_OS_STORE.to_owned()));
        };
        let token = match os_store.read(Scope::Machine, &record.instance.to_string()) {
            Ok(Some(token)) => token,
            Ok(None) => {
                let reason = SkipReason::Unreadable(NOT_IN_OS_STORE.to_owned());
                return MachineTokenLookup::Skipped(reason);
            }
            Err(cause) => {
                return MachineTokenLookup::Skipped(SkipReason::Unreadable(cause.to_string()));
            }
        };
        MachineTokenLookup::Usable(Credential {
            token,
            source: Source::MachineOsStore { machine_id },
        })
    }

    /// Returns the user token of `instance`. `skipped` is why the machine
    /// token was not returned, if the device has one.
    fn read_user_token(
        &self,
        instance: &DashboardUrl,
        skipped: Option<SkipReason>,
    ) -> Result<Credential, CredentialStoreError> {
        let config = AuthConfig::load(&self.auth_config)?;
        let Some(entry @ Instance { accounts: [Some(account)], .. }) = config.instance(instance)
        else {
            let cause = match skipped {
                Some(reason) => Cause::MachineTokenSkipped(reason),
                None => Cause::NoToken,
            };
            return Err(CredentialStoreError::for_instance(instance, cause));
        };
        if let Some(expire_at) = account.expire_at
            && expire_at <= Utc::now()
        {
            return Err(CredentialStoreError::for_instance(instance, Cause::Expired(expire_at)));
        }
        if let AccountToken::Cleartext(token) = &account.token {
            let source = Source::UserFile { machine_token_skipped: skipped };
            return Ok(Credential { token: token.clone(), source });
        }
        let os_store = match self.user_os_store() {
            Ok(os_store) => os_store,
            Err(OsStoreErrorReason::Unavailable(detail)) => {
                let cause = Cause::OsStoreUnavailable(detail);
                return Err(CredentialStoreError::for_instance(instance, cause));
            }
            Err(_) => {
                let cause = Cause::OsStoreUnavailable(DISABLED.to_owned());
                return Err(CredentialStoreError::for_instance(instance, cause));
            }
        };
        let read = os_store
            .read(Scope::User, &entry.url)
            .map_err(|cause| CredentialStoreError::for_instance(instance, cause))?;
        let Some(token) = read else {
            return Err(CredentialStoreError::for_instance(instance, Cause::OsStoreItemMissing));
        };
        let source = Source::UserOsStore { machine_token_skipped: skipped };
        Ok(Credential { token, source })
    }

    fn user_os_store(&self) -> Result<&OsStore, OsStoreErrorReason> {
        if self.user_os_store_disabled {
            return Err(OsStoreErrorReason::Disabled);
        }
        self.os_store
            .as_ref()
            .ok_or_else(|| OsStoreErrorReason::Unavailable(NO_OS_STORE.to_owned()))
    }

    fn store_machine_token(
        &self,
        instance: &DashboardUrl,
        token: &str,
        info: &TokenInfo,
        machine_id: Option<&str>,
    ) -> Result<StoreOutcome, CredentialStoreError> {
        let Some(machine_id) = machine_id else {
            return Err(CredentialStoreError::for_instance(instance, Cause::MissingMachineId));
        };
        let Some(path) = self.machine_credential.as_deref() else {
            let err = io::Error::new(io::ErrorKind::NotFound, "no machine-wide config directory");
            return Err(CredentialStoreError::for_instance(instance, Cause::Write(err)));
        };
        let account = instance.to_string();
        let written = self
            .os_store
            .as_ref()
            .ok_or_else(|| OsStoreErrorReason::Unavailable(NO_OS_STORE.to_owned()))
            .and_then(|os_store| {
                os_store.check_available(Scope::Machine)?;
                os_store
                    .write(Scope::Machine, &account, token, /* interactive */ false)
                    .map_err(OsStoreErrorReason::refused)
            });
        let reason = match written {
            Ok(()) => None,
            Err(OsStoreErrorReason::UnsupportedCaller) => {
                return Err(CredentialStoreError::for_instance(
                    instance,
                    Cause::MachineStoreDenied,
                ));
            }
            Err(reason) => Some(reason),
        };
        let file_token = reason.is_some().then_some(token);
        MachineCredential::new(instance, info, machine_id, file_token).save(path)?;
        Ok(match reason {
            None => StoreOutcome::MachineOsStore,
            Some(reason) => StoreOutcome::MachineFile { reason, path: path.to_path_buf() },
        })
    }

    fn store_user_token(
        &self,
        instance: &DashboardUrl,
        token: &str,
        info: &TokenInfo,
        interactive: bool,
    ) -> Result<StoreOutcome, CredentialStoreError> {
        if token.starts_with(MACHINE_TOKEN_PREFIX) {
            return Err(CredentialStoreError::for_instance(instance, Cause::MachineStoreDenied));
        }
        let mut config = AuthConfig::load(&self.auth_config)?;
        let key = config
            .instance(instance)
            .map_or_else(|| instance.to_string(), |entry| entry.url.clone());
        let written = self.user_os_store().and_then(|os_store| {
            os_store.check_available(Scope::User)?;
            os_store
                .write(Scope::User, &key, token, interactive)
                .map_err(OsStoreErrorReason::refused)
        });
        let (token, outcome) = match written {
            Ok(()) => (AccountToken::InOsStore, StoreOutcome::UserOsStore),
            Err(reason) => (
                AccountToken::Cleartext(token.to_owned()),
                StoreOutcome::UserFile { reason, path: self.auth_config.clone() },
            ),
        };
        let account = Account {
            expire_at: info.expire_at,
            token,
            token_name: info.name.clone(),
            token_type: info.token_type.clone(),
            workspace_id: info.workspace_id,
        };
        match config.instance_mut(instance) {
            Some(entry) => entry.accounts = [Some(account)],
            None => config
                .instances
                .push(Instance { accounts: [Some(account)], ..Instance::new(instance) }),
        }
        config.save(&self.auth_config)?;
        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use gg_fs::temp::TempDir;
    use yare::parameterized;

    use super::*;
    use crate::os_store::{InMemoryOsStore, OsStoreFailure};
    use crate::{Location, TokenType};

    const AFTER_LOGOUT: &str = include_str!("../tests/fixtures/auth_config/after_logout.yaml");
    const CONFIG_SET_WITHOUT_LOGIN: &str =
        include_str!("../tests/fixtures/auth_config/config_set_without_login.yaml");
    const LOGIN_CLEARTEXT_NO_EXPIRY: &str =
        include_str!("../tests/fixtures/auth_config/login_cleartext_no_expiry.yaml");
    const MARKER: &str = include_str!("../tests/fixtures/auth_config/marker.yaml");
    const TWO_INSTANCES_WITH_EXPIRY: &str =
        include_str!("../tests/fixtures/auth_config/two_instances_with_expiry.yaml");

    const SAAS: &str = "https://dashboard.gitguardian.com";
    const BAD_INSTANCE: &str = "- accounts:\n  - token: t\n    token_name: n\n    type: \
                                personal_access_token\n    workspace_id: 1\n  url: not a url\n";
    const ONPREM: &str = "https://gitguardian.example.com";

    fn url(text: &str) -> DashboardUrl {
        text.parse().unwrap()
    }

    fn store_with(dir: &TempDir, contents: &str, os_store: Option<OsStore>) -> CredentialStore {
        let auth_config = dir.path().join("auth_config.yaml");
        std::fs::write(&auth_config, contents).unwrap();
        CredentialStore {
            auth_config,
            machine_credential: Some(dir.path().join("machine_credential.yaml")),
            os_store,
            user_os_store_disabled: false,
        }
    }

    fn in_memory(items: &[(&str, &str)]) -> Option<OsStore> {
        let os_store = OsStore::InMemory(InMemoryOsStore::default());
        for (account, token) in items {
            os_store.write(Scope::User, account, token, false).unwrap();
        }
        Some(os_store)
    }

    fn failing(failure: OsStoreFailure) -> Option<OsStore> {
        Some(OsStore::InMemory(InMemoryOsStore::failing(failure)))
    }

    fn with_machine_token(token: &str) -> Option<OsStore> {
        let os_store = OsStore::InMemory(InMemoryOsStore::default());
        os_store.write(Scope::Machine, SAAS, token, false).unwrap();
        Some(os_store)
    }

    fn machine_record(token: Option<&str>) -> MachineCredential {
        MachineCredential {
            version: 1,
            instance: url(SAAS),
            expire_at: Some(Utc.with_ymd_and_hms(2099, 1, 1, 0, 0, 0).unwrap()),
            scopes: vec!["scan".to_owned()],
            workspace_id: 7,
            name: "machine".to_owned(),
            machine_id: "machine-1".to_owned(),
            token: token.map(str::to_owned),
        }
    }

    fn write_record(dir: &TempDir, record: &MachineCredential) {
        let text = serde_yaml_ng::to_string(record).unwrap();
        std::fs::write(dir.path().join("machine_credential.yaml"), text).unwrap();
    }

    fn request(required_scopes: &[&str]) -> ReadRequest {
        ReadRequest {
            required_scopes: required_scopes.iter().map(|scope| (*scope).to_owned()).collect(),
            explicit_token: Some("explicit".to_owned()),
            ..ReadRequest::default()
        }
    }

    /// GIVEN a cleartext token in the file
    /// WHEN it is read
    /// THEN it is returned from the file
    #[test]
    fn read_returns_cleartext_token() {
        let dir = TempDir::new("credential_store").unwrap();
        let store = store_with(&dir, LOGIN_CLEARTEXT_NO_EXPIRY, None);

        let credential = store.read(&url(SAAS), &ReadRequest::default()).unwrap();

        let expected = Credential {
            token: "cleartext-token".to_owned(),
            source: Source::UserFile { machine_token_skipped: None },
        };
        assert_eq!(credential, expected);
    }

    /// GIVEN the marker in the file and the token in the OS store
    /// WHEN it is read
    /// THEN the token is returned from the OS store
    #[test]
    fn read_returns_token_from_os_store() {
        let dir = TempDir::new("credential_store").unwrap();
        let store = store_with(&dir, MARKER, in_memory(&[(SAAS, "stored-token")]));

        let credential = store.read(&url(SAAS), &ReadRequest::default()).unwrap();

        let expected = Credential {
            token: "stored-token".to_owned(),
            source: Source::UserOsStore { machine_token_skipped: None },
        };
        assert_eq!(credential, expected);
    }

    /// GIVEN the marker in the file and an OS store without the item
    /// WHEN it is read
    /// THEN reading fails as a missing item of that instance
    #[test]
    fn read_of_marker_without_item_fails() {
        let dir = TempDir::new("credential_store").unwrap();
        let store = store_with(&dir, MARKER, in_memory(&[]));

        let err = store.read(&url(SAAS), &ReadRequest::default()).unwrap_err();

        assert!(matches!(err.cause(), Cause::OsStoreItemMissing), "{err}");
        assert_eq!(err.instance(), Some(&url(SAAS)));
    }

    /// GIVEN the marker in the file and an OS store that fails
    /// WHEN it is read
    /// THEN reading fails with the failure of the OS store
    #[parameterized(
        unsupported_caller = { OsStoreFailure::UnsupportedCaller, |cause| matches!(cause, Cause::OsStoreDenied) },
        unavailable = { OsStoreFailure::Unavailable, |cause| matches!(cause, Cause::OsStoreUnavailable(_)) },
        locked = { OsStoreFailure::Locked, |cause| matches!(cause, Cause::OsStoreLocked(_)) },
    )]
    fn read_of_marker_with_failing_store_fails(
        failure: OsStoreFailure,
        expected: fn(&Cause) -> bool,
    ) {
        let dir = TempDir::new("credential_store").unwrap();
        let store = store_with(&dir, MARKER, failing(failure));

        let err = store.read(&url(SAAS), &ReadRequest::default()).unwrap_err();

        assert!(expected(err.cause()), "{err}");
        assert_eq!(err.instance(), Some(&url(SAAS)));
    }

    /// GIVEN the marker in the file and the OS store disabled
    /// WHEN it is read
    /// THEN reading fails as unavailable
    #[test]
    fn read_of_marker_with_disabled_store_fails() {
        let dir = TempDir::new("credential_store").unwrap();
        let store = store_with(&dir, MARKER, None);

        let err = store.read(&url(SAAS), &ReadRequest::default()).unwrap_err();

        assert!(matches!(err.cause(), Cause::OsStoreUnavailable(_)), "{err}");
    }

    /// GIVEN a token whose expiry date has passed
    /// WHEN it is read
    /// THEN reading fails as expired
    #[test]
    fn read_rejects_expired_token() {
        let dir = TempDir::new("credential_store").unwrap();
        let expired = TWO_INSTANCES_WITH_EXPIRY.replace("2027-02-10", "2020-02-10");
        let store = store_with(&dir, &expired, None);

        let err = store.read(&url(ONPREM), &ReadRequest::default()).unwrap_err();

        let expire_at = Utc.with_ymd_and_hms(2020, 2, 10, 9, 0, 0).unwrap();
        assert!(matches!(err.cause(), Cause::Expired(date) if *date == expire_at), "{err}");
    }

    /// GIVEN a file without a token for an instance
    /// WHEN that instance is read
    /// THEN reading fails as no token of that instance
    #[parameterized(
        logged_out = { AFTER_LOGOUT, ONPREM },
        unknown_instance = { LOGIN_CLEARTEXT_NO_EXPIRY, ONPREM },
        empty_file = { "", SAAS },
    )]
    fn read_without_token_fails(contents: &str, instance: &str) {
        let dir = TempDir::new("credential_store").unwrap();
        let store = store_with(&dir, contents, None);

        let err = store.read(&url(instance), &ReadRequest::default()).unwrap_err();

        assert!(matches!(err.cause(), Cause::NoToken), "{err}");
        assert_eq!(err.instance(), Some(&url(instance)));
    }

    /// GIVEN no machine token, and a user token that is readable, absent or
    /// behind a disabled OS store
    /// WHEN it is read with an explicit token
    /// THEN the explicit token is returned
    #[parameterized(
        user_token = { LOGIN_CLEARTEXT_NO_EXPIRY },
        no_user_token = { "" },
        unreadable_user_token = { MARKER },
    )]
    fn read_prefers_explicit_token_to_user_token(contents: &str) {
        let dir = TempDir::new("credential_store").unwrap();
        let store = store_with(&dir, contents, None);
        let request =
            ReadRequest { explicit_token: Some("explicit".to_owned()), ..ReadRequest::default() };

        let credential = store.read(&url(SAAS), &request).unwrap();

        let expected = Credential { token: "explicit".to_owned(), source: Source::Explicit };
        assert_eq!(credential, expected);
    }

    /// GIVEN no machine token, an empty explicit token, and a user token or none
    /// WHEN it is read
    /// THEN the explicit token counts as absent: the user token is returned, or
    /// reading fails as no token
    #[parameterized(
        user_token = { LOGIN_CLEARTEXT_NO_EXPIRY },
        no_user_token = { "" },
    )]
    fn read_ignores_empty_explicit_token(contents: &str) {
        let dir = TempDir::new("credential_store").unwrap();
        let store = store_with(&dir, contents, None);
        let request = ReadRequest { explicit_token: Some(String::new()), ..ReadRequest::default() };

        let read = store.read(&url(SAAS), &request);

        if contents.is_empty() {
            let err = read.unwrap_err();
            assert!(matches!(err.cause(), Cause::NoToken), "{err}");
        } else {
            let source = Source::UserFile { machine_token_skipped: None };
            let expected = Credential { token: "cleartext-token".to_owned(), source };
            assert_eq!(read.unwrap(), expected);
        }
    }

    /// GIVEN a machine token in the file, the OS store or both, a user token
    /// and an explicit token
    /// WHEN it is read with a scope the machine token grants
    /// THEN the machine token is returned with the machine id, from the file
    /// when it holds one
    #[parameterized(
        in_file = { Some("gg_mat_file"), None, "gg_mat_file", false },
        in_os_store = { None, with_machine_token("gg_mat_os"), "gg_mat_os", true },
        in_both = { Some("gg_mat_file"), with_machine_token("gg_mat_os"), "gg_mat_file", false },
    )]
    fn read_returns_machine_token(
        in_file: Option<&str>,
        os_store: Option<OsStore>,
        expected_token: &str,
        from_os_store: bool,
    ) {
        let dir = TempDir::new("credential_store").unwrap();
        let store = store_with(&dir, LOGIN_CLEARTEXT_NO_EXPIRY, os_store);
        write_record(&dir, &machine_record(in_file));

        let credential = store.read(&url(SAAS), &request(&["scan"])).unwrap();

        let machine_id = "machine-1".to_owned();
        let source = if from_os_store {
            Source::MachineOsStore { machine_id }
        } else {
            Source::MachineFile { machine_id }
        };
        assert_eq!(credential, Credential { token: expected_token.to_owned(), source });
    }

    /// GIVEN a machine token recorded without scopes
    /// WHEN it is read without required scopes
    /// THEN the machine token is returned
    #[test]
    fn read_without_required_scopes_returns_machine_token_without_scopes() {
        let dir = TempDir::new("credential_store").unwrap();
        let store = store_with(&dir, LOGIN_CLEARTEXT_NO_EXPIRY, None);
        write_record(
            &dir,
            &MachineCredential { scopes: Vec::new(), ..machine_record(Some("mat")) },
        );

        let credential = store.read(&url(SAAS), &request(&[])).unwrap();

        assert_eq!(credential.token, "mat");
    }

    /// GIVEN a machine token that lacks a required scope or has expired, a user
    /// token and an explicit token
    /// WHEN it is read
    /// THEN the user token is returned with the reason the machine token was
    /// skipped
    #[parameterized(
        missing_scope = {
            |_| (), &["scan", "honeytokens:write"],
            SkipReason::MissingScopes(vec!["honeytokens:write".to_owned()])
        },
        no_recorded_scopes = {
            |record| record.scopes.clear(), &["scan"],
            SkipReason::MissingScopes(vec!["scan".to_owned()])
        },
        expired = {
            |record| record.expire_at = Some(Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap()),
            &["scan"],
            SkipReason::Expired(Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap())
        },
    )]
    fn read_skips_machine_token(
        change: fn(&mut MachineCredential),
        required_scopes: &[&str],
        reason: SkipReason,
    ) {
        let dir = TempDir::new("credential_store").unwrap();
        let store = store_with(&dir, LOGIN_CLEARTEXT_NO_EXPIRY, None);
        let mut record = machine_record(Some("mat"));
        change(&mut record);
        write_record(&dir, &record);

        let credential = store.read(&url(SAAS), &request(required_scopes)).unwrap();

        let expected = Credential {
            token: "cleartext-token".to_owned(),
            source: Source::UserFile { machine_token_skipped: Some(reason) },
        };
        assert_eq!(credential, expected);
    }

    /// GIVEN a machine token that lacks a required scope, no user token and an
    /// explicit token
    /// WHEN it is read
    /// THEN reading fails as a skipped machine token
    #[test]
    fn read_of_skipped_machine_token_without_user_token_fails() {
        let dir = TempDir::new("credential_store").unwrap();
        let store = store_with(&dir, "", None);
        write_record(&dir, &machine_record(Some("mat")));

        let err = store.read(&url(SAAS), &request(&["honeytokens:write"])).unwrap_err();

        let reason = SkipReason::MissingScopes(vec!["honeytokens:write".to_owned()]);
        assert!(
            matches!(err.cause(), Cause::MachineTokenSkipped(skipped) if *skipped == reason),
            "{err}"
        );
    }

    /// GIVEN a machine token of another instance, or one the OS store cannot
    /// return, a user token and an explicit token
    /// WHEN it is read
    /// THEN the user token is returned with the reason the machine token was
    /// skipped
    #[parameterized(
        other_instance = {
            |record| record.instance = url(ONPREM),
            with_machine_token("mat"),
            SkipReason::OtherInstance(ONPREM.to_owned())
        },
        without_os_store = {
            |record| record.token = None, None, SkipReason::Unreadable(NO_OS_STORE.to_owned())
        },
        missing_from_os_store = {
            |record| record.token = None,
            in_memory(&[]),
            SkipReason::Unreadable(NOT_IN_OS_STORE.to_owned())
        },
        failing_os_store = {
            |record| record.token = None,
            failing(OsStoreFailure::Locked),
            SkipReason::Unreadable("the OS store is locked: the store is locked".to_owned())
        },
    )]
    fn read_skips_unusable_machine_token(
        change: fn(&mut MachineCredential),
        os_store: Option<OsStore>,
        reason: SkipReason,
    ) {
        let dir = TempDir::new("credential_store").unwrap();
        let store = store_with(&dir, LOGIN_CLEARTEXT_NO_EXPIRY, os_store);
        let mut record = machine_record(Some("mat"));
        change(&mut record);
        write_record(&dir, &record);

        let credential = store.read(&url(SAAS), &request(&["scan"])).unwrap();

        let expected = Credential {
            token: "cleartext-token".to_owned(),
            source: Source::UserFile { machine_token_skipped: Some(reason) },
        };
        assert_eq!(credential, expected);
    }

    /// GIVEN a machine file that is empty or malformed, or names a malformed
    /// instance URL, and an explicit token
    /// WHEN it is read without a user token
    /// THEN reading fails as a skipped machine token that cannot be read
    #[parameterized(
        empty = { "" },
        malformed = { "version: [" },
        malformed_instance = { "version: 1\ninstance: not a url\nmachine_id: machine-1\n" },
    )]
    fn read_with_unreadable_machine_file_fails(contents: &str) {
        let dir = TempDir::new("credential_store").unwrap();
        let store = store_with(&dir, "", None);
        std::fs::write(dir.path().join("machine_credential.yaml"), contents).unwrap();

        let err = store.read(&url(SAAS), &request(&["scan"])).unwrap_err();

        assert!(
            matches!(err.cause(), Cause::MachineTokenSkipped(SkipReason::Unreadable(_))),
            "{err}"
        );
    }

    /// GIVEN a machine token and a user token
    /// WHEN it is read with an explicit token and the machine token skipped
    /// THEN the user token is returned
    #[test]
    fn read_skipping_machine_token_ignores_explicit_token() {
        let dir = TempDir::new("credential_store").unwrap();
        let store = store_with(&dir, LOGIN_CLEARTEXT_NO_EXPIRY, None);
        write_record(&dir, &machine_record(Some("mat")));
        let request = ReadRequest { skip_machine_token: true, ..request(&["scan"]) };

        let credential = store.read(&url(SAAS), &request).unwrap();

        let expected = Credential {
            token: "cleartext-token".to_owned(),
            source: Source::UserFile { machine_token_skipped: None },
        };
        assert_eq!(credential, expected);
    }

    /// GIVEN a request with an explicit token
    /// WHEN it is formatted for debugging
    /// THEN the token is not shown
    #[test]
    fn read_request_debug_hides_explicit_token() {
        let request =
            ReadRequest { explicit_token: Some("secret".to_owned()), ..ReadRequest::default() };

        assert!(!format!("{request:?}").contains("secret"));
    }

    /// GIVEN a file with one logged in and one logged out instance
    /// WHEN the tokens are listed
    /// THEN only the logged in instance is listed, with its details
    #[test]
    fn list_returns_instances_with_token() {
        let dir = TempDir::new("credential_store").unwrap();
        let store = store_with(&dir, AFTER_LOGOUT, None);

        let ListOutcome { tokens, unreadable } = store.list().unwrap();

        assert_eq!(unreadable, []);
        assert_eq!(tokens.len(), 1, "{tokens:?}");
        let token = tokens.first().unwrap();
        assert_eq!(token.instance, url(SAAS));
        assert_eq!(token.info.workspace_id, 42);
        assert_eq!(token.location, Location::UserOsStore);
    }

    /// GIVEN a machine token in the file or in the OS store, and a user token
    /// WHEN the tokens are listed
    /// THEN the machine token is listed first, with its details and machine id,
    /// followed by the user token
    #[parameterized(
        in_file = { Some("gg_mat_file"), false },
        in_os_store = { None, true },
    )]
    fn list_returns_machine_token_first(in_file: Option<&str>, in_os_store: bool) {
        let dir = TempDir::new("credential_store").unwrap();
        let store = store_with(&dir, LOGIN_CLEARTEXT_NO_EXPIRY, None);
        write_record(&dir, &machine_record(in_file));

        let ListOutcome { tokens, unreadable } = store.list().unwrap();

        let machine_id = "machine-1".to_owned();
        let location = if in_os_store {
            Location::MachineOsStore { machine_id }
        } else {
            Location::MachineFile { machine_id }
        };
        let machine_token = StoredToken {
            instance: url(SAAS),
            instance_name: None,
            info: TokenInfo {
                token_type: TokenType::MachineAccessToken,
                expire_at: Some(Utc.with_ymd_and_hms(2099, 1, 1, 0, 0, 0).unwrap()),
                workspace_id: 7,
                name: "machine".to_owned(),
                scopes: vec!["scan".to_owned()],
            },
            location,
        };
        assert_eq!(unreadable, []);
        assert_eq!(tokens.len(), 2, "{tokens:?}");
        assert_eq!(tokens.first(), Some(&machine_token));
        assert_eq!(tokens.get(1).map(|token| &token.location), Some(&Location::UserFile));
    }

    /// GIVEN a readable user token next to an instance with a malformed URL, a
    /// malformed machine file, or a machine file with a malformed instance URL
    /// WHEN the tokens are listed
    /// THEN the readable token is listed, and the other entry is reported as
    /// unreadable with the URL as written or the path of the file
    #[parameterized(
        user_instance = { BAD_INSTANCE, None, true },
        malformed_machine_file = { "", Some("version: ["), false },
        malformed_machine_instance = {
            "", Some("version: 1\ninstance: not a url\nmachine_id: machine-1\n"), false
        },
    )]
    fn list_reports_unreadable_entries(
        extra_instance: &str,
        machine_file: Option<&str>,
        user_instance: bool,
    ) {
        let dir = TempDir::new("credential_store").unwrap();
        let store = store_with(&dir, &format!("{LOGIN_CLEARTEXT_NO_EXPIRY}{extra_instance}"), None);
        let path = dir.path().join("machine_credential.yaml");
        if let Some(contents) = machine_file {
            std::fs::write(&path, contents).unwrap();
        }

        let ListOutcome { tokens, unreadable } = store.list().unwrap();

        assert_eq!(tokens.len(), 1, "{tokens:?}");
        assert_eq!(tokens.first().map(|token| &token.instance), Some(&url(SAAS)));
        match unreadable.as_slice() {
            [UnreadableEntry::UserInstance { url, .. }] if user_instance => {
                assert_eq!(url, "not a url");
            }
            [UnreadableEntry::MachineFile { path: reported, .. }] if !user_instance => {
                assert_eq!(*reported, path);
            }
            other => panic!("{other:?}"),
        }
    }

    /// GIVEN no record, or a record
    /// WHEN the managed instance is read
    /// THEN it is the instance of the record, and none without one
    #[parameterized(
        no_record = { None },
        record = { Some(SAAS) },
    )]
    fn managed_instance_is_the_instance_of_the_record(instance: Option<&str>) {
        let dir = TempDir::new("credential_store").unwrap();
        let store = store_with(&dir, "", None);
        if let Some(instance) = instance {
            write_record(
                &dir,
                &MachineCredential { instance: url(instance), ..machine_record(None) },
            );
        }

        assert_eq!(store.managed_instance().unwrap(), instance.map(url));
    }

    /// GIVEN a malformed machine file, or a record with a malformed instance
    /// URL
    /// WHEN the managed instance is read
    /// THEN reading fails as malformed, naming the file
    #[parameterized(
        malformed_file = { "version: [" },
        malformed_instance = { "version: 1\ninstance: not a url\nmachine_id: machine-1\n" },
    )]
    fn managed_instance_of_malformed_file_fails(contents: &str) {
        let dir = TempDir::new("credential_store").unwrap();
        let store = store_with(&dir, "", None);
        let path = dir.path().join("machine_credential.yaml");
        std::fs::write(&path, contents).unwrap();

        let err = store.managed_instance().unwrap_err();

        assert!(matches!(err.cause(), Cause::Malformed(_)), "{err}");
        assert_eq!(err.path(), Some(path.as_path()));
    }

    /// GIVEN a global lifetime and one for an instance
    /// WHEN the lifetime is asked for that instance, another one and none
    /// THEN the instance has its own lifetime, and the others the global one
    #[parameterized(
        own = { Some("https://dashboard.eu1.gitguardian.com"), Some(30) },
        other = { Some(SAAS), Some(90) },
        global = { None, Some(90) },
    )]
    fn default_token_lifetime_falls_back_to_global(instance: Option<&str>, expected: Option<u32>) {
        let dir = TempDir::new("credential_store").unwrap();
        let store = store_with(&dir, CONFIG_SET_WITHOUT_LOGIN, None);

        let lifetime = store.default_token_lifetime(instance.map(url).as_ref()).unwrap();

        assert_eq!(lifetime, expected);
    }

    fn new_token_info() -> TokenInfo {
        TokenInfo {
            token_type: TokenType::PersonalAccessToken,
            expire_at: None,
            workspace_id: 7,
            name: "new token".to_owned(),
            scopes: vec!["scan".to_owned()],
        }
    }

    fn file_text(store: &CredentialStore) -> String {
        std::fs::read_to_string(&store.auth_config).unwrap()
    }

    /// GIVEN no config file and no OS store
    /// WHEN a token is stored
    /// THEN it is written in clear text, and the outcome names the reason and
    /// the file
    #[test]
    fn store_without_os_store_writes_cleartext() {
        let dir = TempDir::new("credential_store").unwrap();
        let auth_config = dir.path().join("ggshield").join("auth_config.yaml");
        let store = CredentialStore {
            auth_config: auth_config.clone(),
            machine_credential: None,
            os_store: None,
            user_os_store_disabled: false,
        };

        let outcome = store.store(&url(SAAS), "new-token", &new_token_info(), None).unwrap();

        let reason = unavailable_reason(NO_OS_STORE);
        assert_eq!(outcome, StoreOutcome::UserFile { reason, path: auth_config });
        let credential = store.read(&url(SAAS), &ReadRequest::default()).unwrap();
        let expected = Credential {
            token: "new-token".to_owned(),
            source: Source::UserFile { machine_token_skipped: None },
        };
        assert_eq!(credential, expected);
    }

    /// GIVEN an OS store
    /// WHEN a token is stored
    /// THEN the token is written to the OS store and the file holds the marker
    #[test]
    fn store_with_os_store_writes_marker() {
        let dir = TempDir::new("credential_store").unwrap();
        let store = store_with(&dir, "", in_memory(&[]));

        let outcome = store.store(&url(SAAS), "new-token", &new_token_info(), None).unwrap();

        assert_eq!(outcome, StoreOutcome::UserOsStore);
        assert!(file_text(&store).contains("token: __KEYRING__"), "{}", file_text(&store));
        let credential = store.read(&url(SAAS), &ReadRequest::default()).unwrap();
        let expected = Credential {
            token: "new-token".to_owned(),
            source: Source::UserOsStore { machine_token_skipped: None },
        };
        assert_eq!(credential, expected);
    }

    fn unavailable_reason(detail: &str) -> OsStoreErrorReason {
        OsStoreErrorReason::Unavailable(detail.to_owned())
    }

    fn refused_by_store() -> OsStoreErrorReason {
        OsStoreErrorReason::Refused(
            "the OS store is unavailable: the store refused the token".to_owned(),
        )
    }

    /// GIVEN an OS store that fails
    /// WHEN a token is stored
    /// THEN it is written in clear text, and the outcome names the reason
    #[parameterized(
        unsupported_caller = { OsStoreFailure::UnsupportedCaller, OsStoreErrorReason::UnsupportedCaller },
        unavailable = { OsStoreFailure::Unavailable, unavailable_reason("no store is reachable") },
        locked = { OsStoreFailure::Locked, OsStoreErrorReason::Locked },
        write_failed = { OsStoreFailure::WriteFailed, refused_by_store() },
        write_denied = {
            OsStoreFailure::WriteDenied,
            OsStoreErrorReason::Refused(
                "the OS store denies this installation access to the token".to_owned()
            )
        },
    )]
    fn store_with_failing_store_writes_cleartext(
        failure: OsStoreFailure,
        reason: OsStoreErrorReason,
    ) {
        let dir = TempDir::new("credential_store").unwrap();
        let store = store_with(&dir, "", failing(failure));

        let outcome = store.store(&url(SAAS), "new-token", &new_token_info(), None).unwrap();

        let path = store.auth_config.clone();
        assert_eq!(outcome, StoreOutcome::UserFile { reason, path });
        let credential = store.read(&url(SAAS), &ReadRequest::default()).unwrap();
        let expected = Credential {
            token: "new-token".to_owned(),
            source: Source::UserFile { machine_token_skipped: None },
        };
        assert_eq!(credential, expected);
    }

    /// GIVEN a machine token
    /// WHEN it is stored with a dialog
    /// THEN storing fails and no file is written
    #[test]
    fn store_interactive_refuses_machine_token() {
        let dir = TempDir::new("credential_store").unwrap();
        let store = store_with(&dir, "", None);
        let token = format!("{MACHINE_TOKEN_PREFIX}secret");

        let err = store.store_interactive(&url(SAAS), &token, &new_token_info()).unwrap_err();

        assert!(matches!(err.cause(), Cause::MachineStoreDenied), "{err}");
        assert!(!dir.path().join("machine_credential.yaml").exists());
        assert_eq!(file_text(&store), "");
    }

    fn machine_token_info() -> TokenInfo {
        TokenInfo {
            token_type: TokenType::MachineAccessToken,
            expire_at: Some(Utc.with_ymd_and_hms(2099, 1, 1, 0, 0, 0).unwrap()),
            workspace_id: 7,
            name: "machine".to_owned(),
            scopes: vec!["scan".to_owned(), "endpoints:send".to_owned()],
        }
    }

    fn load_record(dir: &TempDir) -> Option<MachineCredential> {
        MachineCredential::load(&dir.path().join("machine_credential.yaml")).unwrap()
    }

    /// GIVEN no OS store, an OS store that accepts the token, or one that does not
    /// WHEN a machine token is stored with a machine id
    /// THEN it is written to the OS store if one accepts it and to the file with
    /// the reason otherwise, the record holds the details, and a read returns
    /// it
    #[parameterized(
        without_os_store = { None, Some(unavailable_reason(NO_OS_STORE)) },
        with_os_store = { in_memory(&[]), None },
        unavailable_os_store = {
            failing(OsStoreFailure::Unavailable), Some(unavailable_reason("no store is reachable"))
        },
        locked_os_store = { failing(OsStoreFailure::Locked), Some(OsStoreErrorReason::Locked) },
        refusing_os_store = {
            failing(OsStoreFailure::WriteFailed), Some(refused_by_store())
        },
        denying_os_store = {
            failing(OsStoreFailure::WriteDenied),
            Some(OsStoreErrorReason::Refused(
                "the OS store denies this installation access to the token".to_owned()
            ))
        },
    )]
    fn store_writes_machine_token(os_store: Option<OsStore>, reason: Option<OsStoreErrorReason>) {
        let dir = TempDir::new("credential_store").unwrap();
        let store = store_with(&dir, "", os_store);

        let outcome = store
            .store(&url(SAAS), "gg_mat_new", &machine_token_info(), Some("machine-1"))
            .unwrap();

        let (expected_outcome, file_token, source) = match reason {
            None => {
                let source = Source::MachineOsStore { machine_id: "machine-1".to_owned() };
                (StoreOutcome::MachineOsStore, None, source)
            }
            Some(reason) => {
                let path = dir.path().join("machine_credential.yaml");
                let source = Source::MachineFile { machine_id: "machine-1".to_owned() };
                (StoreOutcome::MachineFile { reason, path }, Some("gg_mat_new"), source)
            }
        };
        assert_eq!(outcome, expected_outcome);
        let record =
            MachineCredential::new(&url(SAAS), &machine_token_info(), "machine-1", file_token);
        assert_eq!(load_record(&dir), Some(record));
        let credential = store.read(&url(SAAS), &request(&["scan"])).unwrap();
        assert_eq!(credential, Credential { token: "gg_mat_new".to_owned(), source });
        assert_eq!(file_text(&store), "");
    }

    /// GIVEN a machine token in the file
    /// WHEN a new machine token is stored with an OS store that accepts it
    /// THEN the file no longer holds a token
    #[test]
    fn store_of_machine_token_in_os_store_clears_file_token() {
        let dir = TempDir::new("credential_store").unwrap();
        let without_os_store = store_with(&dir, "", None);
        let info = machine_token_info();
        without_os_store.store(&url(SAAS), "gg_mat_old", &info, Some("machine-1")).unwrap();
        let with_os_store = store_with(&dir, "", in_memory(&[]));

        with_os_store.store(&url(SAAS), "gg_mat_new", &info, Some("machine-1")).unwrap();

        assert_eq!(load_record(&dir).unwrap().token, None);
    }

    /// GIVEN a machine token without a machine id, or a caller the OS store
    /// does not serve
    /// WHEN it is stored
    /// THEN storing fails and nothing is written
    #[parameterized(
        missing_machine_id = {
            None, in_memory(&[]), |cause| matches!(cause, Cause::MissingMachineId)
        },
        unsupported_caller = {
            Some("machine-1"),
            failing(OsStoreFailure::UnsupportedCaller),
            |cause| matches!(cause, Cause::MachineStoreDenied)
        },
    )]
    fn store_of_machine_token_fails(
        machine_id: Option<&str>,
        os_store: Option<OsStore>,
        expected: fn(&Cause) -> bool,
    ) {
        let dir = TempDir::new("credential_store").unwrap();
        let store = store_with(&dir, "", os_store);

        let err =
            store.store(&url(SAAS), "gg_mat_new", &machine_token_info(), machine_id).unwrap_err();

        assert!(expected(err.cause()), "{err}");
        assert_eq!(load_record(&dir), None);
        let os_store = store.os_store.as_ref().unwrap();
        assert!(!matches!(os_store.read(Scope::Machine, SAAS), Ok(Some(_))));
    }

    /// GIVEN no machine-wide config directory
    /// WHEN a machine token is stored
    /// THEN storing fails as a write error
    #[test]
    fn store_of_machine_token_without_directory_fails() {
        let dir = TempDir::new("credential_store").unwrap();
        let store = CredentialStore {
            auth_config: dir.path().join("auth_config.yaml"),
            machine_credential: None,
            os_store: None,
            user_os_store_disabled: false,
        };

        let err = store
            .store(&url(SAAS), "gg_mat_new", &machine_token_info(), Some("machine-1"))
            .unwrap_err();

        assert!(matches!(err.cause(), Cause::Write(_)), "{err}");
    }

    /// GIVEN two logged in instances
    /// WHEN a new token is stored for one of them
    /// THEN its token and details change except its scopes, which a user token
    /// drops, its name stays, and the other instance is unchanged
    #[test]
    fn store_replaces_token_of_one_instance() {
        let dir = TempDir::new("credential_store").unwrap();
        let store = store_with(&dir, TWO_INSTANCES_WITH_EXPIRY, None);

        store.store(&url(ONPREM), "new-token", &new_token_info(), None).unwrap();

        let tokens = store.list().unwrap().tokens;
        let onprem = tokens.iter().find(|token| token.instance == url(ONPREM)).unwrap();
        assert_eq!(onprem.instance_name.as_deref(), Some("onprem"));
        assert_eq!(onprem.info, TokenInfo { scopes: Vec::new(), ..new_token_info() });
        let saas = store.read(&url(SAAS), &ReadRequest::default()).unwrap();
        assert_eq!(saas.token, "cleartext-token");
    }

    /// GIVEN an instance URL with a trailing slash that is not in the file
    /// WHEN a token is stored for it
    /// THEN the instance is added without the trailing slash
    #[test]
    fn store_adds_instance() {
        let dir = TempDir::new("credential_store").unwrap();
        let store = store_with(&dir, "", None);

        store
            .store(&url("https://new.example.com/"), "new-token", &new_token_info(), None)
            .unwrap();

        assert!(
            file_text(&store).contains("url: https://new.example.com\n"),
            "{}",
            file_text(&store)
        );
    }

    /// GIVEN a token in the OS store
    /// WHEN it is deleted
    /// THEN the item and the account are removed, and the instance keeps its
    /// entry
    #[test]
    fn delete_removes_token_and_item() {
        let dir = TempDir::new("credential_store").unwrap();
        let store = store_with(&dir, MARKER, in_memory(&[(SAAS, "stored-token")]));

        let outcome = store.delete(&url(SAAS)).unwrap();

        assert_eq!(outcome, DeleteOutcome::Removed);
        let err = store.read(&url(SAAS), &ReadRequest::default()).unwrap_err();
        assert!(matches!(err.cause(), Cause::NoToken), "{err}");
        let os_store = store.os_store.as_ref().unwrap();
        assert_eq!(os_store.read(Scope::User, SAAS).unwrap(), None);
        assert!(file_text(&store).contains("- accounts:\n  - null\n"), "{}", file_text(&store));
    }

    /// GIVEN a token in clear text, and an OS store that can be used, cannot
    /// be used, or refuses the removal
    /// WHEN it is deleted
    /// THEN it is removed from the file, and only a refused removal reports a
    /// token that may stay in the OS store
    #[parameterized(
        usable = { in_memory(&[]), false, DeleteOutcome::Removed },
        disabled = { in_memory(&[]), true, DeleteOutcome::Removed },
        unavailable = { failing(OsStoreFailure::Unavailable), false, DeleteOutcome::Removed },
        locked = {
            failing(OsStoreFailure::Locked),
            false,
            DeleteOutcome::KeptInOsStore(OsStoreErrorReason::Locked)
        },
    )]
    fn delete_removes_cleartext_token(
        os_store: Option<OsStore>,
        disabled: bool,
        expected: DeleteOutcome,
    ) {
        let dir = TempDir::new("credential_store").unwrap();
        let store = CredentialStore {
            user_os_store_disabled: disabled,
            ..store_with(&dir, LOGIN_CLEARTEXT_NO_EXPIRY, os_store)
        };

        let outcome = store.delete(&url(SAAS)).unwrap();

        assert_eq!(outcome, expected);
        let err = store.read(&url(SAAS), &ReadRequest::default()).unwrap_err();
        assert!(matches!(err.cause(), Cause::NoToken), "{err}");
    }

    /// GIVEN a token in clear text, and an earlier token of the same instance in
    /// the OS store
    /// WHEN it is deleted
    /// THEN the earlier token is removed from the OS store
    #[test]
    fn delete_removes_earlier_token_from_os_store() {
        let dir = TempDir::new("credential_store").unwrap();
        let store =
            store_with(&dir, LOGIN_CLEARTEXT_NO_EXPIRY, in_memory(&[(SAAS, "earlier-token")]));

        let outcome = store.delete(&url(SAAS)).unwrap();

        assert_eq!(outcome, DeleteOutcome::Removed);
        let os_store = store.os_store.as_ref().unwrap();
        assert_eq!(os_store.read(Scope::User, SAAS).unwrap(), None);
    }

    /// GIVEN a token behind the marker and an OS store that cannot remove it
    /// WHEN it is deleted
    /// THEN the account is removed from the file, and the outcome reports the
    /// token kept in the OS store with the reason
    #[parameterized(
        disabled = { in_memory(&[(SAAS, "stored-token")]), true, OsStoreErrorReason::Disabled },
        unsupported_caller = {
            failing(OsStoreFailure::UnsupportedCaller), false, OsStoreErrorReason::UnsupportedCaller
        },
        unavailable = {
            failing(OsStoreFailure::Unavailable), false, unavailable_reason("no store is reachable")
        },
        locked = { failing(OsStoreFailure::Locked), false, OsStoreErrorReason::Locked },
    )]
    fn delete_keeps_token_in_os_store(
        os_store: Option<OsStore>,
        disabled: bool,
        reason: OsStoreErrorReason,
    ) {
        let dir = TempDir::new("credential_store").unwrap();
        let store = CredentialStore {
            user_os_store_disabled: disabled,
            ..store_with(&dir, MARKER, os_store)
        };

        let outcome = store.delete(&url(SAAS)).unwrap();

        assert_eq!(outcome, DeleteOutcome::KeptInOsStore(reason));
        assert!(file_text(&store).contains("- accounts:\n  - null\n"), "{}", file_text(&store));
    }

    /// GIVEN a file without a token for an instance
    /// WHEN that instance is deleted
    /// THEN the outcome reports no stored token, and the file is unchanged
    #[parameterized(
        logged_out = { AFTER_LOGOUT, ONPREM },
        unknown_instance = { LOGIN_CLEARTEXT_NO_EXPIRY, ONPREM },
    )]
    fn delete_without_token_changes_nothing(contents: &str, instance: &str) {
        let dir = TempDir::new("credential_store").unwrap();
        let store = store_with(&dir, contents, None);

        let outcome = store.delete(&url(instance)).unwrap();

        assert_eq!(outcome, DeleteOutcome::NotStored);
        assert_eq!(file_text(&store), contents);
    }

    /// GIVEN a lifetime set globally, for a known instance and for a new one
    /// WHEN the lifetime is read back
    /// THEN each is returned, and the new instance has an entry without an
    /// account
    #[parameterized(
        global = { None, Some(14) },
        known_instance = { Some(SAAS), Some(5) },
        new_instance = { Some("https://new.example.com"), Some(5) },
    )]
    fn set_setting_changes_lifetime(instance: Option<&str>, days: Option<u32>) {
        let dir = TempDir::new("credential_store").unwrap();
        let store = store_with(&dir, LOGIN_CLEARTEXT_NO_EXPIRY, None);
        let instance = instance.map(url);

        store
            .set_setting(Setting::DefaultTokenLifetime { instance: instance.clone(), days })
            .unwrap();

        assert_eq!(store.default_token_lifetime(instance.as_ref()).unwrap(), days);
    }

    /// GIVEN an instance that is not in the file
    /// WHEN its lifetime is removed
    /// THEN the file is unchanged
    #[test]
    fn set_setting_removal_for_unknown_instance_changes_nothing() {
        let dir = TempDir::new("credential_store").unwrap();
        let store = store_with(&dir, LOGIN_CLEARTEXT_NO_EXPIRY, None);
        let setting = Setting::DefaultTokenLifetime { instance: Some(url(ONPREM)), days: None };

        store.set_setting(setting).unwrap();

        assert_eq!(file_text(&store), LOGIN_CLEARTEXT_NO_EXPIRY);
    }

    /// GIVEN `GGSHIELD_NO_KEYRING` and a machine token in the OS store
    /// WHEN a user token is stored and the machine token is read
    /// THEN the user token is written in clear text, and the machine token is
    /// read from the OS store
    #[test]
    fn no_keyring_switch_applies_to_user_tokens_only() {
        let dir = TempDir::new("credential_store").unwrap();
        let store = CredentialStore {
            user_os_store_disabled: true,
            ..store_with(&dir, "", with_machine_token("gg_mat_os"))
        };
        write_record(&dir, &machine_record(None));

        let outcome = store.store(&url(SAAS), "new-token", &new_token_info(), None).unwrap();
        let credential = store.read(&url(SAAS), &request(&["scan"])).unwrap();

        let path = store.auth_config.clone();
        assert_eq!(outcome, StoreOutcome::UserFile { reason: OsStoreErrorReason::Disabled, path });
        let source = Source::MachineOsStore { machine_id: "machine-1".to_owned() };
        assert_eq!(credential, Credential { token: "gg_mat_os".to_owned(), source });
    }
}
