use chrono::{DateTime, Utc};

/// The details of a token, as returned by the GitGuardian API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenInfo {
    /// The `type` of the token.
    pub token_type: String,
    /// The expiry date of the token, if it has one.
    pub expire_at: Option<DateTime<Utc>>,
    /// The workspace the token belongs to.
    pub workspace_id: u64,
    /// The name of the token.
    pub name: String,
    /// The scopes the token grants. The store records them for a machine token
    /// only.
    pub scopes: Vec<String>,
}
