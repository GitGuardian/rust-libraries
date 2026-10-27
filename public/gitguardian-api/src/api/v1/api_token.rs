use bytes::Bytes;
use http::{Method, Request, Response, StatusCode};
use serde::Serialize;
use uuid::Uuid;

use crate::error::{ApiError, BuildError};
use crate::models::api_token::ordering::ApiTokenOrdering;
use crate::models::api_token::scope::ApiTokenScope;
use crate::models::api_token::status::ApiTokenStatus;
use crate::models::api_token::r#type::ApiTokenType;
use crate::models::api_token::{ApiToken, CreatedApiToken};
use crate::models::page::Page;
use crate::models::pagination::{Cursor, Pagination};
use crate::util::json::{expect_json, expect_json_page, expect_no_content, json_body};
use crate::util::url::set_query;
use crate::{ApiCall, ApiConfig, Paginated};

/// Retrieve details of the current API token.
///
/// `GET /v1/api_tokens/self`, answering `200` Current token details or `401` Invalid API
/// key.
#[derive(Clone, Debug, Default)]
pub struct RetrieveCurrentApiToken;

impl ApiCall for RetrieveCurrentApiToken {
    type Output = ApiToken;

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let url = config.endpoint("api_tokens/self")?;
        Ok(config.request(Method::GET, &url).body(Bytes::new())?)
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_json(response, StatusCode::OK)
    }
}

/// Retrieve details of an API token.
///
/// `GET /v1/api_tokens/{token_id}`, answering `200` Current token details, `401` Invalid
/// API key or `404` API token not found.
#[derive(Clone, Debug)]
pub struct RetrieveApiToken {
    /// Id of the token.
    pub token_id: Uuid,
}

impl RetrieveApiToken {
    pub fn new(token_id: Uuid) -> Self {
        Self { token_id }
    }
}

impl ApiCall for RetrieveApiToken {
    type Output = ApiToken;

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let url = config.endpoint(&format!("api_tokens/{}", self.token_id))?;
        Ok(config.request(Method::GET, &url).body(Bytes::new())?)
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_json(response, StatusCode::OK)
    }
}

#[derive(Clone, Debug, Serialize)]
struct ListApiTokensQueryParam<'a> {
    #[serde(flatten)]
    page: &'a Pagination,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<ApiTokenStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    member_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    creator_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scopes: Option<ApiTokenScope>,
    #[serde(skip_serializing_if = "Option::is_none")]
    search: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ordering: Option<ApiTokenOrdering>,
}

/// List API tokens.
///
/// List all the tokens in the workspace, some filters are available and described below.
///
/// `GET /v1/api_tokens`, answering `200` API tokens list, `400` Invalid data or `401`
/// Invalid API key.
#[derive(Clone, Debug, Default)]
pub struct ListApiTokens {
    pub page: Pagination,
    /// Status of the token.
    pub status: Option<ApiTokenStatus>,
    pub member_id: Option<u32>,
    pub creator_id: Option<u32>,
    /// Tokens with one of the following scopes.
    pub scopes: Option<ApiTokenScope>,
    /// Search tokens based on their name.
    pub search: Option<String>,
    /// Sort the results by their field value. The default sort is ASC, DESC if the field
    /// is preceded by a `-`.
    pub ordering: Option<ApiTokenOrdering>,
}

impl ApiCall for ListApiTokens {
    type Output = Page<ApiToken>;

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let mut url = config.endpoint("api_tokens")?;
        set_query(
            &mut url,
            &ListApiTokensQueryParam {
                page: &self.page,
                status: self.status,
                member_id: self.member_id,
                creator_id: self.creator_id,
                scopes: self.scopes.clone(),
                search: self.search.as_deref(),
                ordering: self.ordering,
            },
        )?;
        Ok(config.request(Method::GET, &url).body(Bytes::new())?)
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_json_page(response, StatusCode::OK)
    }
}

impl Paginated for ListApiTokens {
    fn set_cursor(&mut self, cursor: Cursor) {
        self.page.cursor = Some(cursor);
    }
}

#[derive(Clone, Debug, Serialize)]
struct CreateApiTokenBody<'a> {
    name: &'a str,
    #[serde(rename = "type")]
    token_type: ApiTokenType,
    scopes: &'a [ApiTokenScope],
    lifetime: u32,
}

/// Create an API token.
///
/// `POST /v1/api_tokens`, answering `201` API token created, `400` Invalid data, `401`
/// Invalid API key, `403` Permission denied or `409` Another token is already being
/// created for this member or workspace.
#[derive(Clone, Debug)]
pub struct CreateApiToken {
    /// Name of the token. It must be unique among the active tokens of the same type: per
    /// member for a personal access token, per workspace for a service account token.
    pub name: String,
    /// Type of the token to create. It must match the type of the token used to
    /// authenticate the call.
    pub token_type: ApiTokenType,
    /// Scopes granted to the new token. They cannot exceed the scopes of the token used
    /// to perform the call. Some scopes contain others, so the new token can come back
    /// with more scopes than requested: asking for `incidents:write` also grants
    /// `incidents:read`.
    pub scopes: Vec<ApiTokenScope>,
    /// Number of days before the token expires, starting from its creation.
    pub lifetime: u32,
}

impl CreateApiToken {
    pub fn new(
        name: impl Into<String>,
        token_type: ApiTokenType,
        scopes: Vec<ApiTokenScope>,
        lifetime: u32,
    ) -> Self {
        Self { name: name.into(), token_type, scopes, lifetime }
    }
}

impl ApiCall for CreateApiToken {
    type Output = CreatedApiToken;

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let url = config.endpoint("api_tokens")?;
        json_body(
            config.request(Method::POST, &url),
            &CreateApiTokenBody {
                name: &self.name,
                token_type: self.token_type,
                scopes: &self.scopes,
                lifetime: self.lifetime,
            },
        )
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_json(response, StatusCode::CREATED)
    }
}

/// Revoke the current API token.
///
/// `DELETE /v1/api_tokens/self`, answering `204` Token revocation confirmation or `401`
/// Invalid API key.
#[derive(Clone, Debug, Default)]
pub struct RevokeCurrentApiToken;

impl ApiCall for RevokeCurrentApiToken {
    type Output = ();

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let url = config.endpoint("api_tokens/self")?;
        Ok(config.request(Method::DELETE, &url).body(Bytes::new())?)
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_no_content(response, StatusCode::NO_CONTENT)
    }
}

/// Revoke an API token.
///
/// `DELETE /v1/api_tokens/{token_id}`, answering `204` Token revocation confirmation,
/// `401` Invalid API key or `404` API token not found.
#[derive(Clone, Debug)]
pub struct RevokeApiToken {
    pub token_id: Uuid,
}

impl RevokeApiToken {
    pub fn new(token_id: Uuid) -> Self {
        Self { token_id }
    }
}

impl ApiCall for RevokeApiToken {
    type Output = ();

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let url = config.endpoint(&format!("api_tokens/{}", self.token_id))?;
        Ok(config.request(Method::DELETE, &url).body(Bytes::new())?)
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_no_content(response, StatusCode::NO_CONTENT)
    }
}
