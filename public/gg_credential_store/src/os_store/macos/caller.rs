use super::launcher::Launcher;
use super::security_sys::{SecCode, SecRequirement};
use crate::Cause;

/// Whether the running program is signed by GitGuardian.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Caller {
    Signed,
    Unsigned,
}

impl Caller {
    /// Returns whether the running program is signed by GitGuardian as
    /// ggshield or ggshield-py. A program whose signature cannot be checked is
    /// unsigned.
    pub(super) fn current() -> Self {
        match Self::satisfies_any(&Launcher::ALL.map(|launcher| launcher.requirement)) {
            Ok(true) => Self::Signed,
            Ok(false) => Self::Unsigned,
            Err(cause) => {
                tracing::warn!("the signature of this program cannot be checked: {cause}");
                Self::Unsigned
            }
        }
    }

    fn satisfies_any(requirements: &[&str]) -> Result<bool, Cause> {
        let code = SecCode::current()?;
        for text in requirements {
            if code.satisfies(&SecRequirement::new(text)?)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use yare::parameterized;

    use super::*;

    /// GIVEN the test binary, which GitGuardian did not sign
    /// WHEN the caller is determined
    /// THEN it is unsigned
    #[test]
    fn test_binary_is_unsigned() {
        assert_eq!(Caller::current(), Caller::Unsigned);
    }

    /// GIVEN requirements the test binary meets or does not meet
    /// WHEN they are checked
    /// THEN the result is true if one of them is met
    #[parameterized(
        always = { &["always"], true },
        never = { &["never"], false },
        never_then_always = { &["never", "always"], true },
        none = { &[], false },
    )]
    fn satisfies_any_requirement(requirements: &[&str], expected: bool) {
        assert_eq!(Caller::satisfies_any(requirements).unwrap(), expected);
    }
}
