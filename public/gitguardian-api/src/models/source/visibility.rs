use serde::Serialize;

/// Visibility status of a source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SourceVisibility {
    Public,
    Private,
    Internal,
}
