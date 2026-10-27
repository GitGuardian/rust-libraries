use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
/// Pagination cursor holding the base64 encoded pagination parameters
pub struct Cursor(String);

impl Cursor {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for Cursor {
    fn from(value: String) -> Self {
        Self(value)
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Pagination {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<Cursor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_page: Option<u8>,
}
