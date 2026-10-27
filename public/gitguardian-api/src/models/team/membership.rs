use serde::Deserialize;

use crate::models::team::incident_permission::IncidentPermission;
use crate::models::team::permission::TeamPermission;

/// Membership of a member in a team of a GitGuardian workspace.
#[derive(Clone, Debug, Deserialize)]
pub struct TeamMembership {
    pub id: u32,
    pub member_id: u32,
    pub team_id: u32,
    pub is_team_leader: bool,
    pub team_permission: TeamPermission,
    pub incident_permission: IncidentPermission,
}
