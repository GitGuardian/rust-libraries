use bytes::Bytes;
use http::{Method, Request, Response, StatusCode};
use serde::Serialize;

use crate::error::{ApiError, BuildError};
use crate::models::jwt::Jwt;
use crate::util::json::{expect_json, json_body};
use crate::{ApiCall, ApiConfig};

/// Body of a JWT creation request
#[derive(Clone, Debug, Serialize)]
struct CreateJwtBody<'a> {
    audience: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    audience_type: Option<&'a str>,
}

/// Create a JSON Web Token.
///
/// Create a short lived JWT for authentication to specific GitGuardian services,
/// including HasMySecretLeaked.
///
/// `POST /v1/auth/jwt`, answering `200` Created JWT, `400` Invalid data or `401` Invalid
/// API key.
#[derive(Clone, Debug)]
pub struct CreateJwt {
    /// Audience of the JWT.
    pub audience: String,
    /// Type of audience.
    pub audience_type: Option<String>,
}

impl CreateJwt {
    pub fn new(audience: impl Into<String>) -> Self {
        Self { audience: audience.into(), audience_type: None }
    }
}

impl ApiCall for CreateJwt {
    type Output = Jwt;

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let url = config.endpoint("auth/jwt")?;
        json_body(
            config.request(Method::POST, &url),
            &CreateJwtBody {
                audience: &self.audience,
                audience_type: self.audience_type.as_deref(),
            },
        )
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_json(response, StatusCode::OK)
    }
}
