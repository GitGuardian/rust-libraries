//! The `gitguardian_api` crate provides types for requests and models of GitGuardian's API.
//! It is HTTP-client-agnostic. See the `gitguardian-client` crate for higher-level wrappers
//! for individual HTTP frameworks like `ureq` and `reqwest`.
//! Each route is represented by a struct that implements [`ApiCall`].
//! Client implementations use [`ApiCall`] to build requests and parse responses.
//! [`Paginated`] allows for iterating over paginated responses with standard iterators.
pub mod api;
pub mod constants;
pub mod error;
pub mod models;
pub(crate) mod util;

pub use bytes;
pub use chrono;
pub use http;
pub use uuid;

use bytes::Bytes;
use http::header::{AUTHORIZATION, HeaderMap, HeaderValue};
use http::{Method, Request, Response};
use url::Url;

use crate::constants::DEFAULT_BASE_URI;
use crate::error::{ApiError, BuildError, ConfigError};
use crate::models::pagination::Cursor;

pub trait ApiCall {
    type Output;

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError>;

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError>;
}

pub trait Paginated: ApiCall {
    fn set_cursor(&mut self, cursor: Cursor);
}

/// Base URI and API key every call is built against.
///
/// The GitGuardian API uses API keys to authenticate requests, sent as
/// `Token <api-key>` in the `Authorization` header.
#[derive(Clone, Debug)]
pub struct ApiConfig {
    base: Url,
    authorization: HeaderValue,
    headers: HeaderMap,
}

impl ApiConfig {
    pub fn new(api_key: &str) -> Result<Self, ConfigError> {
        Self::with_base_uri(api_key, DEFAULT_BASE_URI)
    }

    pub fn with_base_uri(api_key: &str, base_uri: &str) -> Result<Self, ConfigError> {
        let mut authorization =
            HeaderValue::from_str(&format!("Token {api_key}")).map_err(|_| ConfigError::ApiKey)?;
        authorization.set_sensitive(true);
        Ok(Self { base: Url::parse(base_uri)?, authorization, headers: HeaderMap::new() })
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        &mut self.headers
    }

    pub fn request(&self, method: Method, url: &Url) -> http::request::Builder {
        let mut builder = Request::builder().method(method).uri(url.as_str());
        if let Some(headers) = builder.headers_mut() {
            for (name, value) in &self.headers {
                headers.append(name, value.clone());
            }
            headers.insert(AUTHORIZATION, self.authorization.clone());
        }
        builder
    }

    pub fn endpoint(&self, path: &str) -> Result<Url, BuildError> {
        Ok(self.base.join(path)?)
    }
}
