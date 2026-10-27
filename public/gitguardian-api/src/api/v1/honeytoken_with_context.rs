use bytes::Bytes;
use http::{Method, Request, Response, StatusCode};
use serde::Serialize;

use crate::error::{ApiError, BuildError};
use crate::models::honeytoken::HoneytokenWithContext;
use crate::models::honeytoken::r#type::HoneytokenType;
use crate::util::json::{expect_json, json_body};
use crate::{ApiCall, ApiConfig};

/// Body of a honeytoken with context creation request
#[derive(Clone, Debug, Serialize)]
struct CreateHoneytokenWithContextBody<'a> {
    name: &'a str,
    #[serde(rename = "type")]
    honeytoken_type: HoneytokenType,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    filename: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    language: Option<&'a str>,
    project_extensions: String,
}

/// Create a honeytoken within a context.
///
/// This endpoint allows you to create a honeytoken of a given type within a context. The
/// context is a realistic file in which your honeytoken is inserted.
///
/// If `language`, `project_extensions` and `filename` are not provided, a random context
/// will be generated.
///
/// `POST /v1/honeytokens/with-context`, answering `200` Honeytoken within a context
/// created, `400` Invalid data, `401` Invalid API key, `403` Forbidden Call, `409` Data
/// conflict or `503` API under maintenance.
#[derive(Clone, Debug)]
pub struct CreateHoneytokenWithContext {
    pub name: String,
    pub honeytoken_type: HoneytokenType,
    pub description: Option<String>,
    /// Filename to use for the context.
    pub filename: Option<String>,
    pub language: Option<String>,
    /// File extensions that can be used for the context.
    pub project_extensions: Vec<String>,
}

impl CreateHoneytokenWithContext {
    pub fn new(name: impl Into<String>, honeytoken_type: HoneytokenType) -> Self {
        Self {
            name: name.into(),
            honeytoken_type,
            description: None,
            filename: None,
            language: None,
            project_extensions: Vec::new(),
        }
    }
}

impl ApiCall for CreateHoneytokenWithContext {
    type Output = HoneytokenWithContext;

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let url = config.endpoint("honeytokens/with-context")?;
        json_body(
            config.request(Method::POST, &url),
            &CreateHoneytokenWithContextBody {
                name: &self.name,
                honeytoken_type: self.honeytoken_type.clone(),
                description: self.description.as_deref(),
                filename: self.filename.as_deref(),
                language: self.language.as_deref(),
                project_extensions: self.project_extensions.join(","),
            },
        )
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_json(response, StatusCode::OK)
    }
}
