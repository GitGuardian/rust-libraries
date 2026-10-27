use serde::Deserialize;

/// Where the secret of an incident is still present.
#[derive(Clone, Debug, Deserialize)]
pub struct SecretPresence {
    pub files_requiring_code_fix: u32,
    pub files_pending_merge: u32,
    pub files_fixed: u32,
    pub outside_vcs: u32,
    pub removed_outside_vcs: u32,
    pub in_vcs: u32,
    pub removed_in_vcs: u32,
}
