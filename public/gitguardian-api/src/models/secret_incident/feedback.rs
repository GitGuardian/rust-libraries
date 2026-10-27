use chrono::{DateTime, Utc};
use serde::Deserialize;

/// Feedback provided on a secret incident.
#[derive(Clone, Debug, Deserialize)]
pub struct Feedback {
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Only if the feedback has been provided from the dashboard.
    #[serde(default)]
    pub member_id: Option<u32>,
    /// Feedback author's e-mail address.
    pub email: String,
    #[serde(default)]
    pub answers: Vec<Answer>,
}

/// Answer given as part of a feedback.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Answer {
    Boolean { field_ref: String, field_label: String, boolean: bool },
    Text { field_ref: String, field_label: String, text: String },
}
