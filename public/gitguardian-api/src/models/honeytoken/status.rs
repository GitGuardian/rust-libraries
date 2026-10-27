use serde::Deserialize;

/// Status of a honeytoken.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum HoneytokenStatus {
    Active,
    Revoked,
    Triggered,
    #[serde(untagged)]
    Other(String),
}
