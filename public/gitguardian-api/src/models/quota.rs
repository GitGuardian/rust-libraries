use chrono::NaiveDate;
use serde::Deserialize;

/// Quota overview of a GitGuardian workspace.
#[derive(Clone, Debug, Deserialize)]
pub struct QuotaOverview {
    pub content: Quota,
}

/// Scanning calls available for a token.
///
/// Quota is shared between all tokens of a workspace.
#[derive(Clone, Debug, Deserialize)]
pub struct Quota {
    pub count: u32,
    pub limit: u32,
    pub remaining: u32,
    pub since: NaiveDate,
}
