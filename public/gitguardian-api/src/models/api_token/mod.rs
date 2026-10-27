use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::models::api_token::scope::ApiTokenScope;
use crate::models::api_token::status::ApiTokenStatus;
use crate::models::api_token::r#type::ApiTokenType;

pub mod ordering;
pub mod scope;
pub mod status;
pub mod r#type;

/// Details of an API Token.
#[derive(Clone, Debug, Deserialize)]
pub struct ApiToken {
    /// Id of API token.
    pub id: Uuid,
    /// Name of API token.
    pub name: String,
    /// Workspace id.
    pub workspace_id: u32,
    #[serde(rename = "type")]
    pub token_type: ApiTokenType,
    pub status: ApiTokenStatus,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub last_used_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub expire_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub revoked_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub member_id: Option<u32>,
    #[serde(default)]
    pub creator_id: Option<u32>,
    #[serde(default)]
    pub scopes: Vec<ApiTokenScope>,
}

/// Details of a freshly created API Token, along with its clear text key.
#[derive(Clone, Debug, Deserialize)]
pub struct CreatedApiToken {
    #[serde(flatten)]
    pub token: ApiToken,
    /// The clear text key of the new token. Use it in the `Authorization: Token <key>`
    /// header for subsequent calls to the GitGuardian API. It is returned by this
    /// endpoint only and cannot be retrieved afterwards, so store it before discarding
    /// the response.
    pub key: String,
}
