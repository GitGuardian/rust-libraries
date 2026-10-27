use serde::Deserialize;

use crate::models::r#match::Match;
use crate::models::validity::Validity;

/// Issue found in a scanned [`Document`](crate::models::document::Document).
#[derive(Clone, Debug, Deserialize)]
pub struct PolicyBreak {
    /// Type of detected policy.
    #[serde(rename = "type")]
    pub break_type: String,
    /// Name of failing policy.
    pub policy: String,
    /// Validity of the found secret.
    #[serde(default)]
    pub validity: Option<Validity>,
    /// List of secret matches.
    pub matches: Vec<Match>,
    /// Indicates whether the secret is known by the GitGuardian dashboard.
    #[serde(default)]
    pub known_secret: bool,
}
