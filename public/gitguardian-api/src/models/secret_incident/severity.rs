use serde::Deserialize;

/// Severity of a secret incident.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
    Unknown,
    #[serde(untagged)]
    Other(String),
}
