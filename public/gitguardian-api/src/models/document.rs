use serde::Serialize;

use crate::models::document_location::DocumentLocation;

/// Document to scan.
///
/// The request body shouldn't exceed 1MB.
#[derive(Clone, Debug, Serialize)]
pub struct Document {
    /// Content of the file.
    pub document: String,
    /// Name of the file, example: `.env`. The API limits it to 256 characters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    /// Optional location data, displayed with occurrences found.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<DocumentLocation>,
}

impl Document {
    /// Any `0` byte in the content is replaced with the ASCII substitute character to preserve content length.
    pub fn new(content: impl Into<String>) -> Self {
        Self { document: content.into().replace('\0', "\u{1a}"), filename: None, location: None }
    }

    pub fn with_filename(mut self, filename: impl Into<String>) -> Self {
        self.filename = Some(filename.into());
        self
    }

    pub fn with_location(mut self, location: DocumentLocation) -> Self {
        self.location = Some(location);
        self
    }
}
