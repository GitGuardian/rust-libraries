use serde::Serialize;

/// Criticality of a source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SourceCriticality {
    Critical,
    High,
    Medium,
    Low,
    Unknown,
}
