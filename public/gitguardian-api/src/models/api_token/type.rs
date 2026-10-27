use serde::{Deserialize, Serialize};

/// Type of an API token.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ApiTokenType {
    PersonalAccessToken,
    ServiceAccount,
}
