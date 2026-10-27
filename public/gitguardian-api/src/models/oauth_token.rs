use chrono::{DateTime, Utc};
use serde::Deserialize;

/// Personal access token issued by the OAuth token endpoint.
#[derive(Clone, Debug, Deserialize)]
pub struct OAuthToken {
    /// The issued personal access token. Use it in the `Authorization: Token
    /// <access_token>` header for subsequent calls to the GitGuardian API.
    pub access_token: String,
    pub token_type: TokenType,
    /// Seconds until the access token expires. `null` if the token never expires.
    #[serde(default)]
    pub expires_in: Option<u64>,
    /// GitGuardian token type.
    #[serde(rename = "type")]
    pub token_kind: String,
    /// Token name.
    pub name: String,
    /// ID of the GitGuardian workspace the token belongs to.
    pub account_id: u32,
    /// Expiration date of the token (`null` if it never expires).
    #[serde(default)]
    pub expire_at: Option<DateTime<Utc>>,
    /// Scopes granted to the token.
    #[serde(default)]
    pub scope: Vec<String>,
    /// Raw token value (legacy alias of `access_token`, kept for backwards
    /// compatibility).
    pub key: String,
    /// `true` if the requested lifetime was capped by a workspace policy and the token
    /// expires earlier than requested.
    pub expire_at_downsized: bool,
}

/// Type of the issued token.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[non_exhaustive]
pub enum TokenType {
    Bearer,
}
