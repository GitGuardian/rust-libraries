use serde::Deserialize;

/// Reason a secret incident was ignored.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum IgnoreReason {
    TestCredential,
    FalsePositive,
    LowRisk,
    Invalid,
    #[serde(untagged)]
    Other(String),
}
