use serde::Serialize;

/// Sort the results by their field value. The default sort is ASC, DESC if the field is
/// preceded by a `-`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub enum ApiTokenOrdering {
    #[serde(rename = "created_at")]
    CreatedAt,
    #[serde(rename = "-created_at")]
    CreatedAtDesc,
    #[serde(rename = "last_used_at")]
    LastUsedAt,
    #[serde(rename = "-last_used_at")]
    LastUsedAtDesc,
    #[serde(rename = "expire_at")]
    ExpireAt,
    #[serde(rename = "-expire_at")]
    ExpireAtDesc,
    #[serde(rename = "revoked_at")]
    RevokedAt,
    #[serde(rename = "-revoked_at")]
    RevokedAtDesc,
}
