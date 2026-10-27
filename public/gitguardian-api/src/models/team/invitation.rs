use serde::Deserialize;

use crate::models::team::incident_permission::IncidentPermission;
use crate::models::team::permission::TeamPermission;

/// Invitation to a team of a GitGuardian workspace.
#[derive(Clone, Debug, Deserialize)]
pub struct TeamInvitation {
    pub id: u32,
    pub invitation_id: u32,
    pub team_id: u32,
    pub is_team_leader: bool,
    pub team_permission: TeamPermission,
    pub incident_permission: IncidentPermission,
}
