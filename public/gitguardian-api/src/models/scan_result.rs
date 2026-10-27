use serde::Deserialize;

use crate::models::policy_break::PolicyBreak;

/// Result of a content scan.
#[derive(Clone, Debug, Deserialize)]
pub struct ScanResult {
    /// Number of policy breaks on the scanned document.
    pub policy_break_count: u32,
    /// Policies checked on this document.
    pub policies: Vec<String>,
    /// List of policy breaks.
    pub policy_breaks: Vec<PolicyBreak>,
}
