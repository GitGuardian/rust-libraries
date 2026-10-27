use serde::Deserialize;

/// Tag set on a honeytoken by GitGuardian.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum HoneytokenTag {
    PubliclyExposed,
    #[serde(untagged)]
    Other(String),
}
