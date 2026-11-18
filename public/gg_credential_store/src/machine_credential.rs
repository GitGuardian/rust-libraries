use std::path::{Path, PathBuf};
use std::{fmt, io};

use chrono::{DateTime, Utc};
use gg_fs::locations::{Dir, Locations};
use gg_fs::{ReadOptions, Trust};
use serde::{Deserialize, Serialize};

const FILE_NAME: &str = "machine_credential.yaml";

#[cfg(feature = "system-config-override")]
const SYSTEM_CONFIG_DIR_VAR: &str = "GG_SYSTEM_CONFIG_DIR";

/// The content of `machine_credential.yaml`.
#[derive(PartialEq, Eq, Deserialize, Serialize)]
pub(crate) struct MachineCredential {
    pub(crate) version: u32,
    /// The URL as written in the file.
    pub(crate) instance: String,
    #[serde(default)]
    pub(crate) expire_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub(crate) scopes: Vec<String>,
    pub(crate) machine_id: String,
    /// The machine token, if no OS store holds it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) token: Option<String>,
}

#[expect(dead_code, reason = "the machine token store uses it")]
impl MachineCredential {
    /// Returns the path of the file, or `None` if the machine-wide config
    /// directory cannot be determined.
    pub(crate) fn path() -> Option<PathBuf> {
        #[cfg(feature = "system-config-override")]
        if let Some(dir) = std::env::var_os(SYSTEM_CONFIG_DIR_VAR) {
            return Some(PathBuf::from(dir).join(FILE_NAME));
        }
        let dir = Locations::current().dir(Dir::SystemConfig).ok()?;
        Some(dir.join(FILE_NAME))
    }

    /// Returns the record in the file at `path`, or `None` if the file does
    /// not exist, is not trusted, cannot be read or is malformed.
    pub(crate) fn load(path: &Path) -> Option<Self> {
        let trust = if cfg!(any(test, feature = "system-config-override")) {
            Trust::Caller
        } else {
            Trust::RootOnly
        };
        let options = ReadOptions { trust, ..ReadOptions::default() };
        let text = match gg_fs::read_to_string_with(path, &options) {
            Ok(text) => text,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return None,
            Err(err) => {
                tracing::warn!("ignoring the machine token: {err}");
                return None;
            }
        };
        serde_yaml_ng::from_str(&text)
            .inspect_err(|err| match err.location() {
                Some(at) => tracing::warn!(
                    "ignoring the machine token: {}: malformed at line {}",
                    path.display(),
                    at.line()
                ),
                None => tracing::warn!("ignoring the machine token: {}: malformed", path.display()),
            })
            .ok()
    }
}

impl fmt::Debug for MachineCredential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MachineCredential")
            .field("version", &self.version)
            .field("instance", &self.instance)
            .field("expire_at", &self.expire_at)
            .field("scopes", &self.scopes)
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
            instance: "https://dashboard.gitguardian.com".to_owned(),
            expire_at: Some(Utc.with_ymd_and_hms(2027, 2, 10, 9, 0, 0).unwrap()),
            scopes: vec!["scan".to_owned(), "endpoints:send".to_owned()],
            machine_id: "machine-1".to_owned(),
            token: Some("gg_mat_secret".to_owned()),
        }
    }

    fn minimal_record(version: u32) -> MachineCredential {
        MachineCredential {
            version,
            instance: "https://dashboard.gitguardian.com".to_owned(),
            expire_at: None,
            scopes: Vec::new(),
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

        assert_eq!(MachineCredential::load(&path), Some(expected));
    }

    /// GIVEN a record without a token
    /// WHEN it is written as YAML
    /// THEN the text has no token field
    #[test]
    fn missing_token_is_not_written() {
        let text = serde_yaml_ng::to_string(&minimal_record(1)).unwrap();

        assert!(!text.contains("token"), "{text}");
    }

    /// GIVEN no file, or a malformed file
    /// WHEN it is loaded
    /// THEN there is no record
    #[parameterized(
        empty = { Some("") },
        not_yaml = { Some("version: [") },
        missing_machine_id = { Some("version: 1\ninstance: https://dashboard.gitguardian.com\n") },
        missing = { None },
    )]
    fn load_without_record(contents: Option<&str>) {
        let dir = TempDir::new("machine_credential").unwrap();
        let path = match contents {
            Some(contents) => write(&dir, contents),
            None => dir.path().join(FILE_NAME),
        };

        assert_eq!(MachineCredential::load(&path), None);
    }

    /// GIVEN a file that others can write
    /// WHEN it is loaded
    /// THEN there is no record
    #[cfg(unix)]
    #[test]
    fn load_ignores_writable_file() {
        use std::os::unix::fs::PermissionsExt;

        let dir = TempDir::new("machine_credential").unwrap();
        let path = write(&dir, FULL);
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o666)).unwrap();

        assert_eq!(MachineCredential::load(&path), None);
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
