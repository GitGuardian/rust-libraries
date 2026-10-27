use serde::{Deserialize, Serialize};

/// Permission of a team member or team invitation on the incidents of the team.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum IncidentPermission {
    CanView,
    CanEdit,
    FullAccess,
    #[serde(untagged)]
    Other(String),
}
