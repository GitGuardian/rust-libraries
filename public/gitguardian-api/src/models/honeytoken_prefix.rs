use serde::Deserialize;

/// Matching honeytoken hints.
#[derive(Clone, Debug, Deserialize)]
pub struct HoneytokenPrefixMatches {
    #[serde(default)]
    pub matches: Vec<HoneytokenHint>,
}

/// Hint of a honeytoken matching a queried prefix.
#[derive(Clone, Debug, Deserialize)]
pub struct HoneytokenHint {
    /// SHA-256 hex digest of the full HMSL hash.
    pub hint: String,
}
