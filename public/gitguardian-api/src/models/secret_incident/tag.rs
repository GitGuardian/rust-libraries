use serde::Deserialize;

/// Tag set on a secret incident or one of its occurrences.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum IncidentTag {
    DefaultBranch,
    FromHistoricalScan,
    CheckRunSkipFalsePositive,
    CheckRunSkipLowRisk,
    CheckRunSkipTestCred,
    IgnoredInCheckRun,
    Public,
    PubliclyExposed,
    PubliclyLeaked,
    Regression,
    SensitiveFile,
    TestFile,
    FalsePositive,
    Vaulted,
    RevocableByGg,
    #[serde(untagged)]
    Other(String),
}
