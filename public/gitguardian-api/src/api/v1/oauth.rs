use bytes::Bytes;
use http::{Method, Request, Response, StatusCode};
use serde::Serialize;

use crate::error::{ApiError, BuildError};
use crate::models::oauth_token::OAuthToken;
use crate::util::json::{expect_json, form_body};
use crate::{ApiCall, ApiConfig};

/// Body of an OAuth token exchange request
#[derive(Clone, Debug, Serialize)]
struct CreateOAuthTokenBody<'a> {
    grant_type: &'a str,
    code: &'a str,
    redirect_uri: &'a str,
    client_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    client_secret: Option<&'a str>,
    code_verifier: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lifetime: Option<u32>,
}

/// Exchange an authorization code for an access token.
///
/// `POST /v1/oauth/token`, answering `200` Token successfully issued., `400` OAuth 2.0
/// error response or `401` OAuth 2.0 error response when client authentication fails.
#[derive(Clone, Debug)]
pub struct CreateOAuthToken {
    /// The authorization code received from the authorization endpoint.
    pub code: String,
    /// Must match the `redirect_uri` used when requesting the authorization code.
    pub redirect_uri: String,
    /// The OAuth client identifier.
    pub client_id: String,
    /// The OAuth client secret. Required only for confidential clients.
    pub client_secret: Option<String>,
    /// PKCE code verifier ([RFC 7636](https://www.rfc-editor.org/rfc/rfc7636)) whose hash
    /// matches the `code_challenge` sent to the authorization endpoint.
    pub code_verifier: String,
    /// Optional name for the resulting personal access token.
    pub name: Option<String>,
    /// Optional lifetime of the resulting personal access token, in days. `0` means it
    /// never expires.
    pub lifetime: Option<u32>,
}

impl CreateOAuthToken {
    pub fn new(
        code: impl Into<String>,
        redirect_uri: impl Into<String>,
        client_id: impl Into<String>,
        code_verifier: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            redirect_uri: redirect_uri.into(),
            client_id: client_id.into(),
            client_secret: None,
            code_verifier: code_verifier.into(),
            name: None,
            lifetime: None,
        }
    }
}

impl ApiCall for CreateOAuthToken {
    type Output = OAuthToken;

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let url = config.endpoint("oauth/token")?;
        form_body(
            config.request(Method::POST, &url),
            &CreateOAuthTokenBody {
                grant_type: "authorization_code",
                code: &self.code,
                redirect_uri: &self.redirect_uri,
                client_id: &self.client_id,
                client_secret: self.client_secret.as_deref(),
                code_verifier: &self.code_verifier,
                name: self.name.as_deref(),
                lifetime: self.lifetime,
            },
        )
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_json(response, StatusCode::OK)
    }
}
