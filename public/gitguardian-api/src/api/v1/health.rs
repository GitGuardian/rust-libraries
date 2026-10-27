use bytes::Bytes;
use http::{Method, Request, Response, StatusCode};

use crate::error::{ApiError, BuildError};
use crate::models::health_status::HealthStatus;
use crate::util::json::expect_json;
use crate::{ApiCall, ApiConfig};

/// API Health.
///
/// Check the status of the API and your token without spending your quota.
///
/// `GET /v1/health`, answering `200` API on and key valid response, `401` Invalid API key
/// or `503` API under maintenance.
#[derive(Clone, Debug, Default)]
pub struct CheckHealth;

impl ApiCall for CheckHealth {
    type Output = HealthStatus;

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let url = config.endpoint("health")?;
        Ok(config.request(Method::GET, &url).body(Bytes::new())?)
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_json(response, StatusCode::OK)
    }
}
