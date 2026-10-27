use serde::Deserialize;

/// Status of a secret incident.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum IncidentStatus {
    Ignored,
    Triggered,
    Assigned,
    Resolved,
    #[serde(untagged)]
    Other(String),
}
