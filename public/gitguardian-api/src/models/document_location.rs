use serde::Serialize;

/// Optional location data, displayed with occurrences found.
#[derive(Clone, Debug, Serialize)]
pub struct DocumentLocation {
    /// HTTP URL where the document can be found. It is rendered as a clickable link in
    /// the dashboard.
    pub url: String,
}

impl DocumentLocation {
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }
}
