//! The `gitguardian_api` crate provides types for requests and models of GitGuardian's API.
//! It is HTTP-client-agnostic. See the `gitguardian-client` crate for higher-level wrappers
//! for individual HTTP frameworks like `ureq` and `reqwest`.
//! Each route is represented by a struct that implements [`ApiCall`].
//! Client implementations use [`ApiCall`] to build requests and parse responses.
//! [`Paginated`] allows for iterating over paginated responses with standard iterators.
pub mod api;
pub mod error;
pub mod models;
pub(crate) mod util;

pub use bytes;
pub use chrono;
pub use gg_url;
pub use http;
pub use uuid;

use bytes::Bytes;
use gg_url::{ApiUrl, Error as UrlError};
use http::header::{AUTHORIZATION, HeaderMap, HeaderValue};
use http::{Method, Request, Response};
use url::Url;

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

/// API URL and API key every call is built against.
///
/// The GitGuardian API uses API keys to authenticate requests, sent as
/// `Token <api-key>` in the `Authorization` header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApiConfig {
    base: Url,
    authorization: HeaderValue,
    headers: HeaderMap,
}

impl ApiConfig {
    /// Returns a configuration for GitGuardian US.
    pub fn new(api_key: &str) -> Result<Self, ConfigError> {
        Self::with_api_url(api_key, &ApiUrl::default())
    }

    /// Returns a configuration for the API at `api_url`. Plain HTTP is accepted
    /// only for a loopback host.
    pub fn with_api_url(api_key: &str, api_url: &ApiUrl) -> Result<Self, ConfigError> {
        let mut authorization =
            HeaderValue::from_str(&format!("Token {api_key}")).map_err(|_| ConfigError::ApiKey)?;
        authorization.set_sensitive(true);
        let base = api_url.secure_endpoint("/")?;
        let base = Url::parse(base.as_str()).map_err(UrlError::Invalid)?;
        Ok(Self { base, authorization, headers: HeaderMap::new() })
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

#[cfg(test)]
mod tests {
    use gg_url::HttpUrl;
    use yare::parameterized;

    use super::*;
    use crate::api::v1::health::CheckHealth;

    /// GIVEN an API URL as text
    /// WHEN building the health request
    /// THEN the request URL is the API URL followed by `v1/health`
    #[parameterized(
        saas = { "https://api.gitguardian.com", "https://api.gitguardian.com/v1/health" },
        with_version = { "https://api.gitguardian.com/v1/", "https://api.gitguardian.com/v1/health" },
        europe = { "https://api.eu1.gitguardian.com", "https://api.eu1.gitguardian.com/v1/health" },
        self_hosted = { "https://gg.example.com/exposed", "https://gg.example.com/exposed/v1/health" },
        loopback_http = { "http://localhost:3000", "http://localhost:3000/v1/health" },
    )]
    fn builds_request_url(api_url: &str, expected: &str) {
        let config = ApiConfig::with_api_url("api-key", &api_url.parse().unwrap()).unwrap();
        let request = CheckHealth.build(&config).unwrap();
        assert_eq!(request.uri().to_string(), expected);
    }

    /// GIVEN an API URL that ends in `/v1`
    /// WHEN building the health request
    /// THEN the request URL holds `v1` once
    #[parameterized(
        saas = { "https://api.gitguardian.com/v1", "https://api.gitguardian.com/v1/health" },
        trailing_slash = { "https://api.gitguardian.com/v1/", "https://api.gitguardian.com/v1/health" },
        self_hosted = { "https://gg.example.com/exposed/v1", "https://gg.example.com/exposed/v1/health" },
    )]
    fn does_not_repeat_the_version(api_url: &str, expected: &str) {
        let api_url = ApiUrl(api_url.parse::<HttpUrl>().unwrap());
        let config = ApiConfig::with_api_url("api-key", &api_url).unwrap();
        let request = CheckHealth.build(&config).unwrap();
        assert_eq!(request.uri().to_string(), expected);
    }

    /// GIVEN an API key only
    /// WHEN building the health request
    /// THEN the request URL is on the GitGuardian US API
    #[test]
    fn defaults_to_gitguardian_us() {
        let config = ApiConfig::new("api-key").unwrap();
        let request = CheckHealth.build(&config).unwrap();
        assert_eq!(request.uri().to_string(), "https://api.gitguardian.com/v1/health");
    }

    /// GIVEN an API URL built from a plain HTTP URL on a remote host
    /// WHEN creating the configuration
    /// THEN it is refused
    #[test]
    fn refuses_remote_plain_http() {
        let api_url = ApiUrl("http://gg.example.com".parse::<HttpUrl>().unwrap());
        let config = ApiConfig::with_api_url("api-key", &api_url);
        assert_eq!(config, Err(ConfigError::ApiUrl(UrlError::PlaintextRefused)));
    }
}
