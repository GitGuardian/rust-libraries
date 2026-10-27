use serde::Deserialize;
use uuid::Uuid;

/// Custom tag set on a resource.
#[derive(Clone, Debug, Deserialize)]
pub struct CustomTag {
    pub id: Uuid,
    pub key: String,
    #[serde(default)]
    pub value: Option<String>,
}
