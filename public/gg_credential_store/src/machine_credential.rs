use std::path::{Path, PathBuf};
use std::{fmt, io};

use chrono::{DateTime, Utc};
use gg_fs::locations::{Dir, Locations};
use gg_fs::{DirOptions, Mode, ReadOptions, Trust, WriteOptions};
use gg_url::DashboardUrl;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{Cause, CredentialStoreError, Location, SkipReason, StoredToken, TokenInfo, TokenType};

const FILE_NAME: &str = "machine_credential.yaml";
const VERSION: u32 = 1;

/// The content of `machine_credential.yaml`.
#[derive(PartialEq, Eq, Deserialize, Serialize)]
pub(crate) struct MachineCredential {
    pub(crate) version: u32,
    #[serde(with = "instance_url")]
    pub(crate) instance: DashboardUrl,
    #[serde(default)]
    pub(crate) expire_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub(crate) scopes: Vec<String>,
    #[serde(default)]
    pub(crate) workspace_id: u64,
    #[serde(default)]
    pub(crate) name: String,
    pub(crate) machine_id: String,
    /// The machine token, if no OS store holds it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) token: Option<String>,
}

impl MachineCredential {
    /// Returns the record of a machine token. `token` is `None` when an OS
    /// store holds the token.
    pub(crate) fn new(
        instance: &DashboardUrl,
        info: &TokenInfo,
        machine_id: &str,
        token: Option<&str>,
    ) -> Self {
        Self {
            version: VERSION,
            instance: instance.clone(),
            expire_at: info.expire_at,
            scopes: info.scopes.clone(),
            workspace_id: info.workspace_id,
            name: info.name.clone(),
            machine_id: machine_id.to_owned(),
            token: token.map(str::to_owned),
        }
    }

    /// Returns the path of the file, or `None` if the machine-wide config
    /// directory cannot be determined.
    pub(crate) fn path() -> Option<PathBuf> {
        let dir = Locations::current().dir(Dir::SystemConfig).ok()?;
        Some(dir.join(FILE_NAME))
    }

    /// Returns the record in the file at `path`, or `None` if the file does
    /// not exist.
    ///
    /// Fails if the file is not trusted, cannot be read or is malformed.
    pub(crate) fn load(path: &Path) -> Result<Option<Self>, CredentialStoreError> {
        let trust = if cfg!(test) { Trust::Caller } else { Trust::RootOnly };
        let options = ReadOptions { trust, ..ReadOptions::default() };
        let text = match gg_fs::read_to_string_with(path, &options) {
            Ok(text) => text,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(CredentialStoreError::read(err)),
        };
        serde_yaml_ng::from_str(&text)
            .map(Some)
            .map_err(|err| CredentialStoreError::at(path, Cause::Malformed(err.to_string())))
    }

    /// Writes the record to the file at `path`. Every user may read the file.
    ///
    /// Fails with [`Cause::MachineStoreDenied`] if the caller may not write the
    /// file.
    pub(crate) fn save(&self, path: &Path) -> Result<(), CredentialStoreError> {
        let yaml = serde_yaml_ng::to_string(self).map_err(|err| {
            let err = io::Error::new(io::ErrorKind::InvalidData, err.to_string());
            CredentialStoreError::at(path, Cause::Write(err))
        })?;
        let created = match path.parent() {
            Some(dir) => gg_fs::create_dir_all_with(dir, &DirOptions { mode: Mode::WorldReadable }),
            None => Ok(()),
        };
        let options =
            WriteOptions { mode: Mode::WorldReadable, durable: true, ..WriteOptions::default() };
        match created.and_then(|()| gg_fs::write_with(path, yaml, &options)) {
            Err(err) if err.kind() == io::ErrorKind::PermissionDenied => {
                Err(CredentialStoreError::at(err.path(), Cause::MachineStoreDenied))
            }
            result => result.map_err(CredentialStoreError::write),
        }
    }

    /// Returns the token without its secret.
    pub(crate) fn stored_token(&self) -> StoredToken {
        let machine_id = self.machine_id.clone();
        let location = if self.token.is_some() {
            Location::MachineFile { machine_id }
        } else {
            Location::MachineOsStore { machine_id }
        };
        StoredToken {
            instance: self.instance.clone(),
            instance_name: None,
            info: TokenInfo {
                token_type: TokenType::MachineAccessToken,
                expire_at: self.expire_at,
                workspace_id: self.workspace_id,
                name: self.name.clone(),
                scopes: self.scopes.clone(),
            },
            location,
        }
    }

    /// Returns why the token may not serve a read that requires
    /// `required_scopes`, or `None` if it may.
    pub(crate) fn skip_reason(&self, required_scopes: &[String]) -> Option<SkipReason> {
        if let Some(expire_at) = self.expire_at
            && expire_at <= Utc::now()
        {
            return Some(SkipReason::Expired(expire_at));
        }
        let missing: Vec<String> =
            required_scopes.iter().filter(|scope| !self.scopes.contains(scope)).cloned().collect();
        (!missing.is_empty()).then_some(SkipReason::MissingScopes(missing))
    }
}

mod instance_url {
    use serde::de::Error;

    use super::{DashboardUrl, Deserialize, Deserializer, Serializer};

    pub(super) fn serialize<S: Serializer>(
        url: &DashboardUrl,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.collect_str(url)
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<DashboardUrl, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(|err| D::Error::custom(format!("instance URL {text}: {err}")))
    }
}

impl fmt::Debug for MachineCredential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MachineCredential")
            .field("version", &self.version)
            .field("instance", &self.instance)
            .field("expire_at", &self.expire_at)
            .field("scopes", &self.scopes)
            .field("workspace_id", &self.workspace_id)
            .field("name", &self.name)
            .field("machine_id", &self.machine_id)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use gg_fs::temp::TempDir;
    use yare::parameterized;

    use super::*;

    const FULL: &str = "\
version: 1
instance: https://dashboard.gitguardian.com
expire_at: 2027-02-10T09:00:00Z
scopes:
  - scan
  - endpoints:send
workspace_id: 7
name: machine
machine_id: machine-1
token: gg_mat_secret
";

    const MINIMAL: &str = "\
version: 1
instance: https://dashboard.gitguardian.com
machine_id: machine-1
";

    const NEWER: &str = "\
version: 2
instance: https://dashboard.gitguardian.com
machine_id: machine-1
renew_after: 2027-01-10T09:00:00Z
";

    fn full_record() -> MachineCredential {
        MachineCredential {
            version: 1,
            instance: "https://dashboard.gitguardian.com".parse().unwrap(),
            expire_at: Some(Utc.with_ymd_and_hms(2027, 2, 10, 9, 0, 0).unwrap()),
            scopes: vec!["scan".to_owned(), "endpoints:send".to_owned()],
            workspace_id: 7,
            name: "machine".to_owned(),
            machine_id: "machine-1".to_owned(),
            token: Some("gg_mat_secret".to_owned()),
        }
    }

    fn minimal_record(version: u32) -> MachineCredential {
        MachineCredential {
            version,
            instance: "https://dashboard.gitguardian.com".parse().unwrap(),
            expire_at: None,
            scopes: Vec::new(),
            workspace_id: 0,
            name: String::new(),
            machine_id: "machine-1".to_owned(),
            token: None,
        }
    }

    fn write(dir: &TempDir, contents: &str) -> PathBuf {
        let path = dir.path().join(FILE_NAME);
        std::fs::write(&path, contents).unwrap();
        path
    }

    /// GIVEN a file with the given content
    /// WHEN it is loaded
    /// THEN the record holds the fields the crate knows
    #[parameterized(
        full = { FULL, full_record() },
        without_token_and_scopes = { MINIMAL, minimal_record(1) },
        newer_version_with_unknown_field = { NEWER, minimal_record(2) },
    )]
    fn load_reads_known_fields(contents: &str, expected: MachineCredential) {
        let dir = TempDir::new("machine_credential").unwrap();
        let path = write(&dir, contents);

        assert_eq!(MachineCredential::load(&path).unwrap(), Some(expected));
    }

    /// GIVEN a record without a token
    /// WHEN it is written as YAML
    /// THEN the text has no token field
    #[test]
    fn missing_token_is_not_written() {
        let text = serde_yaml_ng::to_string(&minimal_record(1)).unwrap();

        assert!(!text.lines().any(|line| line.starts_with("token:")), "{text}");
    }

    /// GIVEN no file
    /// WHEN it is loaded
    /// THEN there is no record
    #[test]
    fn load_without_file_has_no_record() {
        let dir = TempDir::new("machine_credential").unwrap();

        assert_eq!(MachineCredential::load(&dir.path().join(FILE_NAME)).unwrap(), None);
    }

    /// GIVEN a malformed file
    /// WHEN it is loaded
    /// THEN loading fails as malformed, naming the file
    #[parameterized(
        empty = { "" },
        not_yaml = { "version: [" },
        missing_machine_id = { "version: 1\ninstance: https://dashboard.gitguardian.com\n" },
        malformed_instance = { "version: 1\ninstance: not a url\nmachine_id: machine-1\n" },
    )]
    fn load_of_malformed_file_fails(contents: &str) {
        let dir = TempDir::new("machine_credential").unwrap();
        let path = write(&dir, contents);

        let err = MachineCredential::load(&path).unwrap_err();

        assert!(matches!(err.cause(), Cause::Malformed(_)), "{err}");
        assert_eq!(err.path(), Some(path.as_path()));
    }

    /// GIVEN a file that others can write
    /// WHEN it is loaded
    /// THEN loading fails with a permission error
    #[cfg(unix)]
    #[test]
    fn load_rejects_writable_file() {
        use std::os::unix::fs::PermissionsExt;

        let dir = TempDir::new("machine_credential").unwrap();
        let path = write(&dir, FULL);
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o666)).unwrap();

        let err = MachineCredential::load(&path).unwrap_err();

        assert!(
            matches!(err.cause(), Cause::Read(io) if io.kind() == io::ErrorKind::PermissionDenied),
            "{err}"
        );
    }

    /// GIVEN a record and a missing directory
    /// WHEN it is saved and loaded
    /// THEN the same record is returned
    #[test]
    fn save_round_trips_record() {
        let dir = TempDir::new("machine_credential").unwrap();
        let path = dir.path().join("ggshield").join(FILE_NAME);

        full_record().save(&path).unwrap();

        assert_eq!(MachineCredential::load(&path).unwrap(), Some(full_record()));
    }

    /// GIVEN a record and a missing directory
    /// WHEN it is saved
    /// THEN the directory has mode 0755 and the file mode 0644
    #[cfg(unix)]
    #[test]
    fn save_creates_world_readable_file() {
        use std::os::unix::fs::PermissionsExt;

        let dir = TempDir::new("machine_credential").unwrap();
        let path = dir.path().join("ggshield").join(FILE_NAME);

        full_record().save(&path).unwrap();

        let mode = |path: &Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(path.parent().unwrap()), 0o755);
        assert_eq!(mode(&path), 0o644);
    }

    /// GIVEN a directory the caller may not write
    /// WHEN a record is saved in it
    /// THEN saving fails as denied
    #[cfg(unix)]
    #[test]
    fn save_in_read_only_directory_is_denied() {
        use std::os::unix::fs::PermissionsExt;

        let dir = TempDir::new("machine_credential").unwrap();
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o555)).unwrap();
        if std::fs::File::create(dir.path().join("probe")).is_ok() {
            return;
        }

        let result = full_record().save(&dir.path().join(FILE_NAME));

        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
        let err = result.unwrap_err();
        assert!(matches!(err.cause(), Cause::MachineStoreDenied), "{err}");
    }

    /// GIVEN a record with a token
    /// WHEN it is formatted for debugging
    /// THEN the token is not shown
    #[test]
    fn debug_hides_token() {
        let text = format!("{:?}", full_record());

        assert!(!text.contains("gg_mat_secret"), "{text}");
    }
}
