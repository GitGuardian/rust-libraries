use bytes::Bytes;
use http::{Method, Request, Response, StatusCode};
use serde::Serialize;

use crate::error::{ApiError, BuildError};
use crate::models::honeytoken::Honeytoken;
use crate::models::honeytoken::r#type::HoneytokenType;
use crate::util::json::{expect_json, json_body};
use crate::{ApiCall, ApiConfig};

/// Body of a honeytoken creation request
#[derive(Clone, Debug, Serialize)]
struct CreateHoneytokenBody<'a> {
    name: &'a str,
    #[serde(rename = "type")]
    honeytoken_type: HoneytokenType,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
}

/// Create a honeytoken.
///
/// This endpoint allows you to create a honeytoken of a type.
///
/// If you are using a personal access token, you need to have an access level superior or
/// equal to `manager`.
///
/// `POST /v1/honeytokens`, answering `201` Honeytoken created, `400` Invalid data, `401`
/// Invalid API key, `403` Forbidden Call, `409` Data conflict or `503` API under
/// maintenance.
#[derive(Clone, Debug)]
pub struct CreateHoneytoken {
    pub name: String,
    pub honeytoken_type: HoneytokenType,
    pub description: Option<String>,
}

impl CreateHoneytoken {
    pub fn new(name: impl Into<String>, honeytoken_type: HoneytokenType) -> Self {
        Self { name: name.into(), honeytoken_type, description: None }
    }
}

impl ApiCall for CreateHoneytoken {
    type Output = Honeytoken;

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let url = config.endpoint("honeytokens")?;
        json_body(
            config.request(Method::POST, &url),
            &CreateHoneytokenBody {
                name: &self.name,
                honeytoken_type: self.honeytoken_type.clone(),
                description: self.description.as_deref(),
            },
        )
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_json(response, StatusCode::CREATED)
    }
}
