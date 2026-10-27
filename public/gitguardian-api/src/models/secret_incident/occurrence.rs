use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::models::secret_incident::tag::IncidentTag;
use crate::models::source::Source;

/// Occurrence of a secret incident.
#[derive(Clone, Debug, Deserialize)]
pub struct Occurrence {
    pub id: u32,
    pub incident_id: u32,
    pub kind: OccurrenceKind,
    pub source: Source,
    pub author_name: String,
    /// Email address of the author.
    pub author_info: String,
    pub date: DateTime<Utc>,
    pub url: String,
    #[serde(default)]
    pub matches: Vec<OccurrenceMatch>,
    #[serde(default)]
    pub tags: Vec<IncidentTag>,
    pub incident_name: String,
    #[serde(default)]
    pub sha: Option<String>,
    pub presence: OccurrencePresence,
    #[serde(default)]
    pub filepath: Option<String>,
    #[serde(default)]
    pub change_type: Option<ChangeType>,
}

/// Secret match within an occurrence.
#[derive(Clone, Debug, Deserialize)]
pub struct OccurrenceMatch {
    pub name: String,
    pub indice_start: u32,
    pub indice_end: u32,
    pub pre_line_start: Option<u32>,
    pub pre_line_end: Option<u32>,
    pub post_line_start: Option<u32>,
    pub post_line_end: Option<u32>,
}

/// How an occurrence was detected.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum OccurrenceKind {
    Realtime,
    Historical,
    #[serde(untagged)]
    Other(String),
}

/// Whether the secret is still present in the occurrence.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum OccurrencePresence {
    Present,
    Removed,
    #[serde(untagged)]
    Other(String),
}

/// Kind of change the occurrence was found in.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ChangeType {
    Addition,
    Deletion,
    Context,
    #[serde(untagged)]
    Other(String),
}
