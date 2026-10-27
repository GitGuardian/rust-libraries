use serde::Deserialize;

/// Ticket created on an external service for a secret incident.
#[derive(Clone, Debug, Deserialize)]
pub struct DestinationTicket {
    pub id: String,
    #[serde(rename = "type")]
    pub ticket_type: DestinationTicketType,
    pub link: String,
}

/// External service a destination ticket lives on.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DestinationTicketType {
    JiraCloud,
    JiraDataCenter,
    Servicenow,
    #[serde(untagged)]
    Other(String),
}
