use serde::{Deserialize, Serialize};

/// Permission of a team member or team invitation on the team.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum TeamPermission {
    CanManage,
    CannotManage,
}
