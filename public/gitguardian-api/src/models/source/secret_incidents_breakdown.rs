use serde::Deserialize;

/// Secret incidents of a source, split between open and closed ones.
#[derive(Clone, Debug, Deserialize)]
pub struct SecretIncidentsBreakdown {
    pub open_secret_incidents: SecretIncidentsCount,
    pub closed_secret_incidents: SecretIncidentsCount,
}

/// Number of secret incidents, split by severity.
#[derive(Clone, Debug, Deserialize)]
pub struct SecretIncidentsCount {
    pub total: u32,
    pub severity_breakdown: SeverityBreakdown,
}

/// Number of secret incidents per severity.
#[derive(Clone, Debug, Deserialize)]
pub struct SeverityBreakdown {
    pub critical: u32,
    pub high: u32,
    pub medium: u32,
    pub low: u32,
    pub info: u32,
    pub unknown: u32,
}
