use bytes::Bytes;
use http::{Method, Request, Response, StatusCode};

use crate::error::{ApiError, BuildError};
use crate::models::quota::QuotaOverview;
use crate::util::json::expect_json;
use crate::{ApiCall, ApiConfig};

/// Quota overview.
///
/// Check available scanning calls for this token. Quota is shared between all tokens of a
/// workspace.
///
/// `GET /v1/quotas`, answering `200` Quota Overview, `401` Invalid API key or `503` API
/// under maintenance.
#[derive(Clone, Debug, Default)]
pub struct RetrieveQuotas;

impl ApiCall for RetrieveQuotas {
    type Output = QuotaOverview;

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let url = config.endpoint("quotas")?;
        Ok(config.request(Method::GET, &url).body(Bytes::new())?)
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_json(response, StatusCode::OK)
    }
}
