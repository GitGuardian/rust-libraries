use std::path::{Path, PathBuf};
use std::{fmt, io};

use chrono::{DateTime, Utc};
use gg_fs::locations::{Dir, Locations};
use gg_fs::{ReadOptions, Trust, WriteOptions};
use gg_url::DashboardUrl;
use serde::{Deserialize, Serialize};

use crate::{Cause, CredentialStoreError, Location, TokenInfo};

const FILE_NAME: &str = "auth_config.yaml";

/// The value of `token` in the file when the token is in the user's OS store.
const OS_STORE_MARKER: &str = "__KEYRING__";

/// The content of `auth_config.yaml`.
#[derive(Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) struct AuthConfig {
    /// The lifetime in days of new tokens, for instances without their own.
    #[serde(default, alias = "default-token-lifetime")]
    pub(crate) default_token_lifetime: Option<u32>,
    #[serde(default)]
    pub(crate) instances: Vec<Instance>,
}

/// A GitGuardian instance and the account logged in to it.
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) struct Instance {
    /// The account logged in to the instance, if any.
    pub(crate) accounts: [Option<Account>; 1],
    /// The lifetime in days of new tokens for this instance. It overrides
    /// [`AuthConfig::default_token_lifetime`].
    #[serde(default, alias = "default-token-lifetime")]
    pub(crate) default_token_lifetime: Option<u32>,
    #[serde(default)]
    pub(crate) name: Option<String>,
    /// The URL as written in the file. Compare it with [`Instance::has_url`].
    pub(crate) url: String,
}

/// The token of an instance and its details.
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) struct Account {
    #[serde(default, alias = "expire-at")]
    pub(crate) expire_at: Option<DateTime<Utc>>,
    pub(crate) token: AccountToken,
    #[serde(alias = "token-name")]
    pub(crate) token_name: String,
    #[serde(rename = "type")]
    pub(crate) token_type: String,
    #[serde(alias = "workspace-id")]
    pub(crate) workspace_id: u64,
}

/// Where the token of an account is.
#[derive(Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(from = "String", into = "String")]
pub(crate) enum AccountToken {
    /// The token, written in the file.
    Cleartext(String),
    /// The token is in the user's OS store.
    InOsStore,
}

impl AuthConfig {
    pub(crate) fn path(locations: &Locations) -> Result<PathBuf, CredentialStoreError> {
        let dir = locations.dir(Dir::Config).map_err(CredentialStoreError::read)?;
        Ok(dir.join(FILE_NAME))
    }

    /// Reads the file at `path`. Returns an empty config if the file does not
    /// exist.
    pub(crate) fn load(path: &Path) -> Result<Self, CredentialStoreError> {
        let options = ReadOptions { trust: Trust::Caller, ..ReadOptions::default() };
        match gg_fs::read_to_string_with(path, &options) {
            Ok(text) => Self::parse(&text)
                .map_err(|err| CredentialStoreError::at(path, Cause::Malformed(err.to_string()))),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(err) => Err(CredentialStoreError::read(err)),
        }
    }

    pub(crate) fn save(&self, path: &Path) -> Result<(), CredentialStoreError> {
        let yaml = serde_yaml_ng::to_string(self).map_err(|err| {
            let err = io::Error::new(io::ErrorKind::InvalidData, err.to_string());
            CredentialStoreError::at(path, Cause::Write(err))
        })?;
        if let Some(dir) = path.parent() {
            gg_fs::create_dir_all(dir).map_err(CredentialStoreError::write)?;
        }
        gg_fs::write_with(path, yaml, &WriteOptions::private()).map_err(CredentialStoreError::write)
    }

    pub(crate) fn parse(text: &str) -> Result<Self, serde_yaml_ng::Error> {
        Ok(serde_yaml_ng::from_str::<Option<Self>>(text)?.unwrap_or_default())
    }

    pub(crate) fn instance(&self, url: &DashboardUrl) -> Option<&Instance> {
        self.instances.iter().find(|instance| instance.has_url(url))
    }

    pub(crate) fn instance_mut(&mut self, url: &DashboardUrl) -> Option<&mut Instance> {
        self.instances.iter_mut().find(|instance| instance.has_url(url))
    }
}

impl Instance {
    /// Returns an instance at `url` without an account or settings.
    pub(crate) fn new(url: &DashboardUrl) -> Self {
        Self { accounts: [None], default_token_lifetime: None, name: None, url: url.to_string() }
    }

    fn has_url(&self, url: &DashboardUrl) -> bool {
        self.url.parse::<DashboardUrl>().is_ok_and(|own| own == *url)
    }
}

impl Account {
    pub(crate) fn info(&self) -> TokenInfo {
        TokenInfo {
            token_type: self.token_type.clone(),
            expire_at: self.expire_at,
            workspace_id: self.workspace_id,
            name: self.token_name.clone(),
            scopes: Vec::new(),
        }
    }
}

impl AccountToken {
    pub(crate) fn location(&self) -> Location {
        match self {
            Self::Cleartext(_) => Location::UserFile,
            Self::InOsStore => Location::UserOsStore,
        }
    }
}

impl From<String> for AccountToken {
    fn from(token: String) -> Self {
        if token == OS_STORE_MARKER { Self::InOsStore } else { Self::Cleartext(token) }
    }
}

impl From<AccountToken> for String {
    fn from(token: AccountToken) -> Self {
        match token {
            AccountToken::Cleartext(token) => token,
            AccountToken::InOsStore => OS_STORE_MARKER.to_owned(),
        }
    }
}

impl fmt::Debug for AccountToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cleartext(_) => f.write_str("Cleartext(<redacted>)"),
            Self::InOsStore => f.write_str("InOsStore"),
        }
    }
}

#[cfg(test)]
mod tests {
    use gg_fs::temp::TempDir;
    use yare::parameterized;

    use super::*;

    const AFTER_LOGOUT: &str = include_str!("../tests/fixtures/auth_config/after_logout.yaml");
    const CONFIG_SET_WITHOUT_LOGIN: &str =
        include_str!("../tests/fixtures/auth_config/config_set_without_login.yaml");
    const LOGIN_CLEARTEXT_NO_EXPIRY: &str =
        include_str!("../tests/fixtures/auth_config/login_cleartext_no_expiry.yaml");
    const MARKER: &str = include_str!("../tests/fixtures/auth_config/marker.yaml");
    const TWO_INSTANCES_WITH_EXPIRY: &str =
        include_str!("../tests/fixtures/auth_config/two_instances_with_expiry.yaml");

    fn url(text: &str) -> DashboardUrl {
        text.parse().unwrap()
    }

    /// GIVEN a file written by ggshield 1.56.1
    /// WHEN it is parsed, written and parsed again
    /// THEN both parses give the same config
    #[parameterized(
        marker = { MARKER },
        two_instances_with_expiry = { TWO_INSTANCES_WITH_EXPIRY },
    )]
    fn golden_file_parses_back_to_same_config(text: &str) {
        let config = AuthConfig::parse(text).unwrap();

        let written = serde_yaml_ng::to_string(&config).unwrap();

        assert_eq!(AuthConfig::parse(&written).unwrap(), config);
    }

    /// GIVEN a file written by ggshield 1.56.1 without an expiry date
    /// WHEN it is parsed and written again
    /// THEN the output is byte for byte the input
    #[parameterized(
        after_logout = { AFTER_LOGOUT },
        config_set_without_login = { CONFIG_SET_WITHOUT_LOGIN },
        login_cleartext_no_expiry = { LOGIN_CLEARTEXT_NO_EXPIRY },
    )]
    fn golden_file_without_expiry_is_written_unchanged(text: &str) {
        let written = serde_yaml_ng::to_string(&AuthConfig::parse(text).unwrap()).unwrap();

        assert_eq!(written, text);
    }

    /// GIVEN an account whose token is the marker
    /// WHEN the file is parsed
    /// THEN the token is in the OS store
    #[test]
    fn marker_means_os_store() {
        let config = AuthConfig::parse(MARKER).unwrap();

        let instance = config.instance(&url("https://dashboard.gitguardian.com")).unwrap();
        let [account] = &instance.accounts;
        assert_eq!(account.as_ref().map(|account| &account.token), Some(&AccountToken::InOsStore));
    }

    /// GIVEN the file after `auth logout`
    /// WHEN it is parsed
    /// THEN the logged out instance has no account and keeps its name
    #[test]
    fn null_account_is_no_account() {
        let config = AuthConfig::parse(AFTER_LOGOUT).unwrap();

        let instance = config.instance(&url("https://gitguardian.example.com")).unwrap();
        assert_eq!(instance.accounts, [None]);
        assert_eq!(instance.name.as_deref(), Some("onprem"));
    }

    /// GIVEN a file with dashes in keys and unknown keys
    /// WHEN it is parsed and written again
    /// THEN the output has the released keys only
    #[test]
    fn tolerant_read_writes_released_format() {
        let text = "default-token-lifetime: 30\nextra: 1\ninstances:\n- accounts:\n  - expire-at: null\n    \
                    token: first\n    token-name: n\n    type: personal_access_token\n    workspace-id: \
                    1\n    extra: 2\n  url: https://dashboard.gitguardian.com\n  extra: 3\n";

        let written = serde_yaml_ng::to_string(&AuthConfig::parse(text).unwrap()).unwrap();

        assert_eq!(
            written,
            "default_token_lifetime: 30\ninstances:\n- accounts:\n  - expire_at: null\n    token: \
             first\n    token_name: n\n    type: personal_access_token\n    workspace_id: 1\n  \
             default_token_lifetime: null\n  name: null\n  url: https://dashboard.gitguardian.com\n"
        );
    }

    /// GIVEN an instance with two accounts
    /// WHEN the file is parsed
    /// THEN parsing fails
    #[test]
    fn second_account_is_rejected() {
        let account = "  - token: t\n    token_name: n\n    type: personal_access_token\n    \
                       workspace_id: 1\n";
        let text = format!(
            "instances:\n- accounts:\n{account}{account}  url: https://dashboard.gitguardian.com\n"
        );

        assert!(AuthConfig::parse(&text).is_err());
    }

    /// GIVEN an instance whose `accounts` key is missing or empty
    /// WHEN the file is parsed
    /// THEN parsing fails
    #[parameterized(
        missing = { "" },
        empty = { "accounts: []\n  " },
    )]
    fn missing_or_empty_accounts_are_rejected(accounts: &str) {
        let text = format!("instances:\n- {accounts}url: https://dashboard.gitguardian.com\n");

        assert!(AuthConfig::parse(&text).is_err());
    }

    /// GIVEN an empty file
    /// WHEN it is parsed
    /// THEN the config is empty
    #[test]
    fn empty_file_is_empty_config() {
        assert_eq!(AuthConfig::parse("").unwrap(), AuthConfig::default());
    }

    /// GIVEN an instance stored without a trailing slash
    /// WHEN it is looked up with a trailing slash
    /// THEN it is found
    #[test]
    fn instance_matches_url_with_trailing_slash() {
        let config = AuthConfig::parse(LOGIN_CLEARTEXT_NO_EXPIRY).unwrap();

        assert!(config.instance(&url("https://dashboard.gitguardian.com/")).is_some());
        assert!(config.instance(&url("https://other.example.com")).is_none());
    }

    /// GIVEN an account with a cleartext token
    /// WHEN it is formatted with Debug
    /// THEN the token does not appear
    #[test]
    fn debug_hides_cleartext_token() {
        let config = AuthConfig::parse(LOGIN_CLEARTEXT_NO_EXPIRY).unwrap();

        let debug = format!("{config:?}");

        assert!(!debug.contains("cleartext-token"), "{debug}");
    }

    /// GIVEN no file and no config directory
    /// WHEN the file is loaded
    /// THEN the config is empty
    #[test]
    fn missing_file_is_empty_config() {
        let dir = TempDir::new("auth_config").unwrap();
        let path = dir.path().join("ggshield").join("auth_config.yaml");

        let config = AuthConfig::load(&path).unwrap();

        assert_eq!(config, AuthConfig::default());
    }

    /// GIVEN a config saved in a directory that does not exist yet
    /// WHEN the file is loaded
    /// THEN the config is the one saved
    #[test]
    fn saved_config_is_loaded_back() {
        let dir = TempDir::new("auth_config").unwrap();
        let path = dir.path().join("ggshield").join("auth_config.yaml");
        let config = AuthConfig::parse(MARKER).unwrap();

        config.save(&path).unwrap();

        assert_eq!(AuthConfig::load(&path).unwrap(), config);
    }

    /// GIVEN a config saved to a new file
    /// WHEN the mode of the file is read
    /// THEN only the owner may read and write it
    #[cfg(unix)]
    #[test]
    fn saved_file_is_private() {
        use std::os::unix::fs::PermissionsExt;

        let dir = TempDir::new("auth_config").unwrap();
        let path = dir.path().join("auth_config.yaml");

        AuthConfig::default().save(&path).unwrap();

        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    /// GIVEN a file that group members may write
    /// WHEN it is loaded
    /// THEN loading fails with a permission error on that file
    #[cfg(unix)]
    #[test]
    fn group_writable_file_is_rejected() {
        use std::os::unix::fs::PermissionsExt;

        let dir = TempDir::new("auth_config").unwrap();
        let path = dir.path().join("auth_config.yaml");
        std::fs::write(&path, LOGIN_CLEARTEXT_NO_EXPIRY).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o664)).unwrap();

        let err = AuthConfig::load(&path).unwrap_err();

        assert!(
            matches!(err.cause(), Cause::Read(io) if io.kind() == io::ErrorKind::PermissionDenied),
            "{err}"
        );
        assert_eq!(err.path(), Some(path.as_path()));
    }

    /// GIVEN a file that is not a valid config
    /// WHEN it is loaded
    /// THEN loading fails as malformed, naming the file
    #[parameterized(
        wrong_type = { b"instances: 3\n" },
        not_utf8 = { b"\xff\xfe" },
    )]
    fn invalid_file_is_malformed(contents: &[u8]) {
        let dir = TempDir::new("auth_config").unwrap();
        let path = dir.path().join("auth_config.yaml");
        std::fs::write(&path, contents).unwrap();

        let err = AuthConfig::load(&path).unwrap_err();

        assert!(matches!(err.cause(), Cause::Malformed(_)), "{err}");
        assert_eq!(err.path(), Some(path.as_path()));
    }
}
