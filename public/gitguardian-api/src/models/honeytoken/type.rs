use serde::{Deserialize, Serialize};

/// Type of a honeytoken.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[non_exhaustive]
pub enum HoneytokenType {
    #[serde(rename = "AWS")]
    Aws,
    #[serde(untagged)]
    Other(String),
}
