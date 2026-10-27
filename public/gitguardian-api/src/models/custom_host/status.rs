use serde::Deserialize;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum CustomHostStatus {
    Valid,
    Unreachable,
    Invalid,
    #[serde(untagged)]
    Other(String),
}
