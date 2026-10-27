use bytes::Bytes;
use http::{Method, Request, Response, StatusCode};
use serde::Serialize;

use crate::error::{ApiError, BuildError};
use crate::models::secret_incident::SecretIncident;
use crate::util::json::expect_json;
use crate::util::url::set_query;
use crate::{ApiCall, ApiConfig};

/// Query parameters of a secret incident retrieval request
#[derive(Clone, Debug, Serialize)]
struct RetrieveSecretIncidentQueryParam {
    #[serde(skip_serializing_if = "Option::is_none")]
    with_occurrences: Option<u8>,
}

/// Retrieve a secret incident.
///
/// Retrieve secret incident detected by the GitGuardian dashboard with its occurrences.
///
/// `GET /v1/incidents/secrets/{incident_id}`, answering `200` Secret incident details,
/// `400` Invalid data, `401` Invalid API key or `503` API under maintenance.
#[derive(Clone, Debug)]
pub struct RetrieveSecretIncident {
    /// The id of the incident to retrieve.
    pub incident_id: u32,
    /// Retrieve a number of occurrences of this incident.
    pub with_occurrences: Option<u8>,
}

impl RetrieveSecretIncident {
    pub fn new(incident_id: u32) -> Self {
        Self { incident_id, with_occurrences: None }
    }
}

impl ApiCall for RetrieveSecretIncident {
    type Output = SecretIncident;

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let mut url = config.endpoint(&format!("incidents/secrets/{}", self.incident_id))?;
        set_query(
            &mut url,
            &RetrieveSecretIncidentQueryParam { with_occurrences: self.with_occurrences },
        )?;
        Ok(config.request(Method::GET, &url).body(Bytes::new())?)
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_json(response, StatusCode::OK)
    }
}
