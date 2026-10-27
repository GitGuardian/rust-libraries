use serde::{Deserialize, Serialize};

/// Scope granted to an API token.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[non_exhaustive]
pub enum ApiTokenScope {
    #[serde(rename = "scan")]
    Scan,
    #[serde(rename = "incidents:read")]
    IncidentsRead,
    #[serde(rename = "incidents:write")]
    IncidentsWrite,
    #[serde(rename = "incidents:share")]
    IncidentsShare,
    #[serde(rename = "members:read")]
    MembersRead,
    #[serde(rename = "members:write")]
    MembersWrite,
    #[serde(rename = "teams:read")]
    TeamsRead,
    #[serde(rename = "teams:write")]
    TeamsWrite,
    #[serde(rename = "audit_logs:read")]
    AuditLogsRead,
    #[serde(rename = "honeytokens:read")]
    HoneytokensRead,
    #[serde(rename = "honeytokens:write")]
    HoneytokensWrite,
    #[serde(rename = "honeytokens:check")]
    HoneytokensCheck,
    #[serde(rename = "api_tokens:read")]
    ApiTokensRead,
    #[serde(rename = "api_tokens:write")]
    ApiTokensWrite,
    #[serde(rename = "ip_allowlist:read")]
    IpAllowlistRead,
    #[serde(rename = "ip_allowlist:write")]
    IpAllowlistWrite,
    #[serde(rename = "sources:read")]
    SourcesRead,
    #[serde(rename = "sources:write")]
    SourcesWrite,
    #[serde(rename = "nhi:send-inventory")]
    NhiSendInventory,
    #[serde(rename = "nhi:write-vault")]
    NhiWriteVault,
    #[serde(rename = "endpoints:send")]
    EndpointsSend,
    #[serde(rename = "ai-discover:send")]
    AiDiscoverSend,
    #[serde(untagged)]
    Other(String),
}
