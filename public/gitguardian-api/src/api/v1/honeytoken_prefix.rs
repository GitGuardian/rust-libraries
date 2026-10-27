use bytes::Bytes;
use http::{Method, Request, Response, StatusCode};
use serde::Serialize;

use crate::error::{ApiError, BuildError};
use crate::models::honeytoken_prefix::HoneytokenPrefixMatches;
use crate::util::json::{expect_json, json_body};
use crate::{ApiCall, ApiConfig};

/// Body of a honeytoken prefix lookup request
#[derive(Clone, Debug, Serialize)]
struct CheckHoneytokenPrefixesBody<'a> {
    prefixes: &'a [String],
}

/// Bulk prefix lookup for honeytoken HMSL hashes.
///
/// `POST /v1/honeytokens/prefixes`, answering `200` Matching honeytoken hints, `400`
/// Invalid request, `401` Invalid API key or `403` Forbidden.
#[derive(Clone, Debug, Default)]
pub struct CheckHoneytokenPrefixes {
    /// List of 5-character lowercase hexadecimal HMSL hash prefixes. Maximum 500
    /// prefixes per request.
    pub prefixes: Vec<String>,
}

impl CheckHoneytokenPrefixes {
    pub fn new(prefixes: Vec<String>) -> Self {
        Self { prefixes }
    }
}

impl FromIterator<String> for CheckHoneytokenPrefixes {
    fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> Self {
        Self::new(iter.into_iter().collect())
    }
}

impl ApiCall for CheckHoneytokenPrefixes {
    type Output = HoneytokenPrefixMatches;

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let url = config.endpoint("honeytokens/prefixes")?;
        json_body(
            config.request(Method::POST, &url),
            &CheckHoneytokenPrefixesBody { prefixes: &self.prefixes },
        )
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_json(response, StatusCode::OK)
    }
}
