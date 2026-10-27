use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::models::invitation::access_level::InvitationAccessLevel;

pub mod access_level;
pub mod ordering;

/// Invitation sent to a user of a GitGuardian workspace.
#[derive(Clone, Debug, Deserialize)]
pub struct Invitation {
    pub id: u32,
    pub email: String,
    pub role: InvitationAccessLevel,
    pub access_level: InvitationAccessLevel,
    pub date: DateTime<Utc>,
}
