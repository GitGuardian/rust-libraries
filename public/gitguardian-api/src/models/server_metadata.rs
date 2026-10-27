use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

/// Public metadata of a GitGuardian instance.
#[derive(Clone, Debug, Deserialize)]
pub struct ServerMetadata {
    pub version: String,
    #[serde(default)]
    pub preferences: HashMap<String, Value>,
    pub secret_scan_preferences: SecretScanPreferences,
    #[serde(default)]
    pub remediation_messages: HashMap<String, String>,
}

/// Limits applied to secret scanning requests.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct SecretScanPreferences {
    pub maximum_documents_per_scan: u32,
    pub maximum_document_size: u32,
}
