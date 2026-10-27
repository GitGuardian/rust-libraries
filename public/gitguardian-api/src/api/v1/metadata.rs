use bytes::Bytes;
use http::{Method, Request, Response, StatusCode};

use crate::error::{ApiError, BuildError};
use crate::models::server_metadata::ServerMetadata;
use crate::util::json::expect_json;
use crate::{ApiCall, ApiConfig};

/// Get public Metadata.
///
/// `GET /v1/metadata`.
#[derive(Clone, Debug, Default)]
pub struct RetrieveMetadata;

impl ApiCall for RetrieveMetadata {
    type Output = ServerMetadata;

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let url = config.endpoint("metadata")?;
        Ok(config.request(Method::GET, &url).body(Bytes::new())?)
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_json(response, StatusCode::OK)
    }
}
