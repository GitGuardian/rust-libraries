use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::models::member::access_level::AccessLevel;

pub mod access_level;
pub mod ordering;

/// Member of a GitGuardian workspace.
#[derive(Clone, Debug, Deserialize)]
pub struct Member {
    pub id: u32,
    pub name: String,
    pub email: String,
    pub role: AccessLevel,
    pub access_level: AccessLevel,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub last_login: Option<DateTime<Utc>>,
}
