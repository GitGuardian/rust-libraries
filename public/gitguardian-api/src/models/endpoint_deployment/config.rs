use serde::{Deserialize, Serialize};

/// Method-specific placement payload.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct DeploymentConfig {
    /// Basename of the on-disk file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    /// Named section/profile the client writes/removes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_name: Option<String>,
}
