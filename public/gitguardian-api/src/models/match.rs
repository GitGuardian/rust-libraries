use serde::Deserialize;

/// Secret match within a [`PolicyBreak`](crate::models::policy_break::PolicyBreak).
#[derive(Clone, Debug, Deserialize)]
pub struct Match {
    /// Matched string of failure.
    pub r#match: String,
    /// Type of match.
    #[serde(rename = "type")]
    pub match_type: String,
    /// Line the match starts on, 1-indexed.
    #[serde(default)]
    pub line_start: Option<u32>,
    /// Line the match ends on, 1-indexed.
    #[serde(default)]
    pub line_end: Option<u32>,
}
