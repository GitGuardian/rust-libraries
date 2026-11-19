use std::ffi::CString;
use std::fs;
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};

use super::security_sys::{SecRequirement, SecStaticCode};
use crate::Cause;

#[cfg(not(gg_credential_store_test_signer))]
const GGSHIELD: &str = include_str!("requirements/ggshield.txt");
#[cfg(not(gg_credential_store_test_signer))]
const GGSHIELD_PY: &str = include_str!("requirements/ggshield-py.txt");
#[cfg(gg_credential_store_test_signer)]
const GGSHIELD: &str = concat!(
    r#"identifier "com.gitguardian.ggshield" and certificate leaf = H""#,
    env!("GG_CREDENTIAL_STORE_TEST_SIGNER"),
    '"'
);
#[cfg(gg_credential_store_test_signer)]
const GGSHIELD_PY: &str = concat!(
    r#"identifier "com.gitguardian.ggshield-py" and certificate leaf = H""#,
    env!("GG_CREDENTIAL_STORE_TEST_SIGNER"),
    '"'
);

/// A program that GitGuardian distributes with ggshield and signs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct Launcher {
    file_name: &'static str,
    pub(super) requirement: &'static str,
}

impl Launcher {
    pub(super) const ALL: [Self; 2] = [
        Self { file_name: "ggshield", requirement: GGSHIELD },
        Self { file_name: "ggshield-py", requirement: GGSHIELD_PY },
    ];

    /// Returns the paths of the running program and of the launchers next to
    /// it that GitGuardian signed.
    pub(super) fn trusted_paths() -> Result<Vec<CString>, Cause> {
        let program = std::env::current_exe()
            .and_then(fs::canonicalize)
            .map_err(|err| Cause::OsStoreUnavailable(format!("no path of this program: {err}")))?;
        let mut paths = vec![program.clone()];
        if let Some(directory) = program.parent() {
            for launcher in Self::ALL {
                if let Some(path) = launcher.signed_in(directory)
                    && !paths.contains(&path)
                {
                    paths.push(path);
                }
            }
        }
        paths
            .into_iter()
            .map(|path| {
                CString::new(path.into_os_string().into_vec())
                    .map_err(|err| Cause::OsStoreUnavailable(err.to_string()))
            })
            .collect()
    }

    /// Returns the path of this launcher in `directory`, if GitGuardian signed
    /// it.
    fn signed_in(&self, directory: &Path) -> Option<PathBuf> {
        let path = fs::canonicalize(directory.join(self.file_name)).ok()?;
        let signed = SecRequirement::new(self.requirement)
            .and_then(|requirement| SecStaticCode::at(&path)?.satisfies(&requirement));
        match signed {
            Ok(signed) => signed.then_some(path),
            Err(cause) => {
                tracing::debug!("{}: the signature cannot be checked: {cause}", path.display());
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use gg_fs::temp::TempDir;
    use yare::parameterized;

    use super::*;

    /// GIVEN the requirements of the GitGuardian launchers
    /// WHEN they are parsed
    /// THEN each one parses
    #[test]
    fn launcher_requirements_parse() {
        for launcher in Launcher::ALL {
            assert!(SecRequirement::new(launcher.requirement).is_ok(), "{launcher:?}");
        }
    }

    enum File {
        Copy(&'static str),
        Script,
        Missing,
    }

    /// GIVEN a file named like a launcher
    /// WHEN it is checked against a requirement
    /// THEN it is found only if it is a program that meets the requirement
    #[parameterized(
        apple_program_for_apple = { File::Copy("/bin/ls"), "anchor apple", true },
        apple_program_for_gitguardian = {
            File::Copy("/bin/ls"),
            Launcher::ALL[0].requirement,
            false
        },
        script = { File::Script, "anchor apple", false },
        missing = { File::Missing, "anchor apple", false },
    )]
    fn launcher_is_found_if_signed(file: File, requirement: &'static str, found: bool) {
        let directory = TempDir::new("launcher").unwrap();
        let path = directory.path().join("ggshield");
        match file {
            File::Copy(program) => drop(fs::copy(program, &path).unwrap()),
            File::Script => fs::write(&path, "#!/bin/sh\n").unwrap(),
            File::Missing => {}
        }
        let launcher = Launcher { file_name: "ggshield", requirement };

        let signed = launcher.signed_in(directory.path());

        assert_eq!(signed.is_some(), found, "{signed:?}");
    }

    /// GIVEN the test binary, with no launcher next to it
    /// WHEN the trusted paths are listed
    /// THEN only the test binary is trusted
    #[test]
    fn running_program_is_trusted() {
        let program = fs::canonicalize(std::env::current_exe().unwrap()).unwrap();

        let paths = Launcher::trusted_paths().unwrap();

        assert_eq!(paths, [CString::new(program.into_os_string().into_vec()).unwrap()]);
    }
}
