use serde::{Deserialize, Serialize};

/// Access level an invitation can be created with.
///
/// A workspace owner cannot be invited, only promoted afterwards.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum InvitationAccessLevel {
    Manager,
    Member,
    Restricted,
}
