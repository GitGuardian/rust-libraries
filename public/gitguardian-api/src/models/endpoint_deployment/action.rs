use serde::Deserialize;

/// Whether the client should write or delete the placement.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DeploymentAction {
    Write,
    Delete,
    #[serde(untagged)]
    Other(String),
}
