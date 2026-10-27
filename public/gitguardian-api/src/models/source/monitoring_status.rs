use serde::{Deserialize, Serialize};

/// Real-time monitoring status of a source.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum MonitoringStatus {
    Active,
    Disabled,
    Unreachable,
    Archived,
    DeletedOnRemote,
    #[serde(untagged)]
    Other(String),
}
