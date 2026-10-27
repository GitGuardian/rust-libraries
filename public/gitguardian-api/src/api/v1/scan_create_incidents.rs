use bytes::Bytes;
use http::{Method, Request, Response, StatusCode};
use serde::Serialize;
use uuid::Uuid;

use crate::error::{ApiError, BuildError};
use crate::models::document::Document;
use crate::models::scan_result::ScanResult;
use crate::util::json::{expect_json, json_body};
use crate::{ApiCall, ApiConfig};

/// Body of a scan and incident creation request
#[derive(Clone, Debug, Serialize)]
struct ScanCreateIncidentsBody<'a> {
    source_uuid: Uuid,
    documents: &'a [Document],
}

/// Scan content and create incidents.
///
/// Scan provided content for hardcoded secrets and create incidents that will be
/// reflected on the GitGuardian dashboard.
///
/// This endpoint is in beta and may be subject to changes in future releases.
///
/// Request body shouldn't exceed 1MB.
///
/// `POST /v1/scan/create-incidents`, answering `200` Successful Scan and Incident
/// Creation, `400` Bad Request, `401` Invalid API key, `403` Forbidden or `503` API under
/// maintenance.
#[derive(Clone, Debug)]
pub struct ScanCreateIncidents {
    /// Identifier of the custom source the incidents are created on.
    pub source_uuid: Uuid,
    pub documents: Vec<Document>,
}

impl ScanCreateIncidents {
    pub fn new(source_uuid: Uuid, documents: Vec<Document>) -> Self {
        Self { source_uuid, documents }
    }
}

impl ApiCall for ScanCreateIncidents {
    type Output = Vec<ScanResult>;

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let url = config.endpoint("scan/create-incidents")?;
        json_body(
            config.request(Method::POST, &url),
            &ScanCreateIncidentsBody { source_uuid: self.source_uuid, documents: &self.documents },
        )
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_json(response, StatusCode::OK)
    }
}
