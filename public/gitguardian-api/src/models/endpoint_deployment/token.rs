use serde::Deserialize;

/// AWS credentials. For `write`, the key to write; for `delete`, the revoked key to match
/// against the on-disk profile before removing it. Present for both actions.
#[derive(Clone, Debug, Deserialize)]
pub struct DeploymentToken {
    #[serde(default)]
    pub access_token_id: Option<String>,
    #[serde(default)]
    pub secret_key: Option<String>,
}
