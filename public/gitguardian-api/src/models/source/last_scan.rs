use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::models::source::scan_status::ScanStatus;

/// Latest historical scan of a source.
#[derive(Clone, Debug, Deserialize)]
pub struct LastScan {
    /// Creation date of this historical scan.
    pub date: DateTime<Utc>,
    pub status: ScanStatus,
    #[serde(default)]
    pub failing_reason: Option<String>,
    #[serde(default)]
    pub commits_scanned: Option<u32>,
    #[serde(default)]
    pub branches_scanned: Option<u32>,
    #[serde(default)]
    pub duration: Option<String>,
}
