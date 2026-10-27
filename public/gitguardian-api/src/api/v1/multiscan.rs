use bytes::Bytes;
use http::{Method, Request, Response, StatusCode};
use serde::Serialize;

use crate::error::{ApiError, BuildError};
use crate::models::document::Document;
use crate::models::scan_result::ScanResult;
use crate::util::json::{expect_json, json_body};
use crate::util::url::set_query;
use crate::{ApiCall, ApiConfig};

/// Query parameters for a multiscan request
#[derive(Clone, Debug, Serialize)]
struct MultiScanQueryParam {
    #[serde(skip_serializing_if = "Option::is_none")]
    all_secrets: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ignore_known_secrets: Option<bool>,
}

/// Multiple content scan.
///
/// Scan provided document contents for policy breaks. Multiple documents are returned by
/// the same index order.
///
/// There should not be more than 20 documents in the payload. Individual documents should
/// not exceed 1MB.
///
/// Quota usage is based on requests and not on the content size. One request to this
/// endpoint will consume 1 API call.
///
/// `POST /v1/multiscan`, answering `200` Successful Scan, `400` Invalid data, `401`
/// Invalid API key, `403` Quota limit reached or `503` API under maintenance.
#[derive(Clone, Debug)]
pub struct MultiScan {
    pub documents: Vec<Document>,
    /// Indicates whether all secrets should be returned.
    pub all_secrets: Option<bool>,
    /// Indicates whether known secrets should be ignored.
    pub ignore_known_secrets: Option<bool>,
}

impl MultiScan {
    pub fn new(documents: Vec<Document>) -> Self {
        Self { documents, all_secrets: None, ignore_known_secrets: None }
    }
}

impl FromIterator<Document> for MultiScan {
    fn from_iter<I: IntoIterator<Item = Document>>(iter: I) -> Self {
        Self::new(iter.into_iter().collect())
    }
}

impl ApiCall for MultiScan {
    type Output = Vec<ScanResult>;

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let mut url = config.endpoint("multiscan")?;
        set_query(
            &mut url,
            &MultiScanQueryParam {
                all_secrets: self.all_secrets,
                ignore_known_secrets: self.ignore_known_secrets,
            },
        )?;
        json_body(config.request(Method::POST, &url), &self.documents)
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_json(response, StatusCode::OK)
    }
}
