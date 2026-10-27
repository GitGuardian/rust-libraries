use serde::Deserialize;

/// Health check response.
#[derive(Clone, Debug, Deserialize)]
pub struct HealthStatus {
    /// API key status.
    pub detail: String,
}
