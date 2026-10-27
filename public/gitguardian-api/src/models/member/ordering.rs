use serde::Serialize;

/// Sort the results by their field value. The default sort is ASC, DESC if the field is
/// preceded by a `-`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub enum MemberOrdering {
    #[serde(rename = "created_at")]
    CreatedAt,
    #[serde(rename = "-created_at")]
    CreatedAtDesc,
    #[serde(rename = "last_login")]
    LastLogin,
    #[serde(rename = "-last_login")]
    LastLoginDesc,
}
