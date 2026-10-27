use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::models::custom_tag::CustomTag;
use crate::models::secret_incident::destination_ticket::DestinationTicket;
use crate::models::secret_incident::detector::Detector;
use crate::models::secret_incident::feedback::Feedback;
use crate::models::secret_incident::ignore_reason::IgnoreReason;
use crate::models::secret_incident::occurrence::Occurrence;
use crate::models::secret_incident::public_exposure::PublicExposure;
use crate::models::secret_incident::secret_presence::SecretPresence;
use crate::models::secret_incident::severity::Severity;
use crate::models::secret_incident::status::IncidentStatus;
use crate::models::secret_incident::tag::IncidentTag;
use crate::models::validity::Validity;

pub mod destination_ticket;
pub mod detector;
pub mod feedback;
pub mod ignore_reason;
pub mod occurrence;
pub mod public_exposure;
pub mod secret_presence;
pub mod severity;
pub mod status;
pub mod tag;

/// Secret incident detected by the GitGuardian dashboard.
#[derive(Clone, Debug, Deserialize)]
pub struct SecretIncident {
    pub id: u32,
    /// Last trigger date of the incident.
    pub date: DateTime<Utc>,
    pub detector: Detector,
    pub secret_id: u32,
    pub secret_hash: String,
    pub hmsl_hash: String,
    pub gitguardian_url: String,
    pub regression: bool,
    pub status: IncidentStatus,
    #[serde(default)]
    pub assignee_id: Option<u32>,
    #[serde(default)]
    pub assignee_email: Option<String>,
    pub occurrences_count: u32,
    pub secret_presence: SecretPresence,
    #[serde(default)]
    pub ignore_reason: Option<IgnoreReason>,
    pub triggered_at: DateTime<Utc>,
    #[serde(default)]
    pub ignored_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub ignorer_id: Option<u32>,
    #[serde(default)]
    pub ignorer_api_token_id: Option<Uuid>,
    #[serde(default)]
    pub resolver_id: Option<u32>,
    #[serde(default)]
    pub resolver_api_token_id: Option<Uuid>,
    pub secret_revoked: bool,
    pub severity: Severity,
    pub validity: Validity,
    #[serde(default)]
    pub resolved_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub share_url: Option<String>,
    #[serde(default)]
    pub tags: Vec<IncidentTag>,
    #[serde(default)]
    pub custom_tags: Vec<CustomTag>,
    #[serde(default)]
    pub feedback_list: Vec<Feedback>,
    pub incident_name: String,
    pub risk_score: u32,
    #[serde(default)]
    pub severity_rule_id: Option<u32>,
    #[serde(default)]
    pub is_vaulted: Option<bool>,
    #[serde(default)]
    pub public_exposure: Option<PublicExposure>,
    #[serde(default)]
    pub destination_tickets: Vec<DestinationTicket>,
    #[serde(default)]
    pub occurrences: Vec<Occurrence>,
}
