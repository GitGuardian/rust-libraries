use bytes::Bytes;
use http::{Method, Request, Response, StatusCode};
use serde::Serialize;

use crate::error::{ApiError, BuildError};
use crate::models::endpoint_deployment::config::DeploymentConfig;
use crate::models::endpoint_deployment::method::DeploymentMethod;
use crate::models::endpoint_deployment::status::DeploymentStatus;
use crate::models::endpoint_deployment::{EndpointDeployment, EndpointDeployments};
use crate::util::json::{expect_json, json_body};
use crate::util::url::set_query;
use crate::{ApiCall, ApiConfig};

/// Machine and OS user the honeytokens are disseminated to.
#[derive(Clone, Debug, Serialize)]
pub struct MachineInfo {
    /// Stable machine identifier (key for the Endpoint).
    pub machine_id: String,
    /// OS-level username on the machine (key for the EndpointUser).
    pub username: String,
    /// Display hostname for the Endpoint.
    pub hostname: String,
}

impl MachineInfo {
    pub fn new(
        machine_id: impl Into<String>,
        username: impl Into<String>,
        hostname: impl Into<String>,
    ) -> Self {
        Self { machine_id: machine_id.into(), username: username.into(), hostname: hostname.into() }
    }
}

/// Custom tag to set on the honeytoken
#[derive(Clone, Debug, Serialize)]
struct CustomTagInput<'a> {
    key: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<&'a str>,
}

/// Body of an endpoint deployment reconcile request
#[derive(Clone, Debug, Serialize)]
struct CreateEndpointDeploymentBody<'a> {
    machine_info: &'a MachineInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    method: Option<DeploymentMethod>,
    #[serde(skip_serializing_if = "Option::is_none")]
    config: Option<&'a DeploymentConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    custom_tags: Vec<CustomTagInput<'a>>,
}

/// Reconcile (plant) honeytokens for a machine/user.
///
/// `POST /v1/honeytokens/endpoint-deployments`, answering `200` The desired honeytoken
/// placements for this machine/user., `400` Invalid data, `401` Invalid API key, `403`
/// Forbidden Call, `409` The request carried an explicit `config` that conflicts with an
/// already-active deployment for the same `(machine, user, method)` or `503` API under
/// maintenance.
#[derive(Clone, Debug)]
pub struct CreateEndpointDeployment {
    pub machine_info: MachineInfo,
    /// Placement method. Optional; defaults to the type's default.
    pub method: Option<DeploymentMethod>,
    /// Optional per-call override of the placement config.
    pub config: Option<DeploymentConfig>,
    /// Optional honeytoken description (applied only on creation).
    pub description: Option<String>,
    /// Custom tags to set on the honeytoken (applied only on creation).
    pub custom_tags: Vec<(String, Option<String>)>,
}

impl CreateEndpointDeployment {
    pub fn new(machine_info: MachineInfo) -> Self {
        Self {
            machine_info,
            method: None,
            config: None,
            description: None,
            custom_tags: Vec::new(),
        }
    }
}

impl ApiCall for CreateEndpointDeployment {
    type Output = EndpointDeployments;

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let url = config.endpoint("honeytokens/endpoint-deployments")?;
        json_body(
            config.request(Method::POST, &url),
            &CreateEndpointDeploymentBody {
                machine_info: &self.machine_info,
                method: self.method.clone(),
                config: self.config.as_ref(),
                description: self.description.as_deref(),
                custom_tags: self
                    .custom_tags
                    .iter()
                    .map(|(key, value)| CustomTagInput { key, value: value.as_deref() })
                    .collect(),
            },
        )
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_json(response, StatusCode::OK)
    }
}

/// Query parameters of an endpoint deployment listing request
#[derive(Clone, Debug, Serialize)]
struct ListEndpointDeploymentsQueryParam<'a> {
    machine_id: &'a str,
    username: &'a str,
}

/// List a machine/user's honeytoken deployments (read-only).
///
/// `GET /v1/honeytokens/endpoint-deployments`, answering `200` The live honeytoken
/// placements for this machine/user (possibly empty)., `400` Invalid data, `401` Invalid
/// API key, `403` Forbidden Call or `503` API under maintenance.
#[derive(Clone, Debug)]
pub struct ListEndpointDeployments {
    pub machine_id: String,
    pub username: String,
}

impl ListEndpointDeployments {
    pub fn new(machine_id: impl Into<String>, username: impl Into<String>) -> Self {
        Self { machine_id: machine_id.into(), username: username.into() }
    }
}

impl ApiCall for ListEndpointDeployments {
    type Output = EndpointDeployments;

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let mut url = config.endpoint("honeytokens/endpoint-deployments")?;
        set_query(
            &mut url,
            &ListEndpointDeploymentsQueryParam {
                machine_id: &self.machine_id,
                username: &self.username,
            },
        )?;
        Ok(config.request(Method::GET, &url).body(Bytes::new())?)
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_json(response, StatusCode::OK)
    }
}

/// Body of an endpoint deployment confirmation request
#[derive(Clone, Debug, Serialize)]
struct ConfirmEndpointDeploymentBody {
    status: DeploymentStatus,
}

/// Confirm a honeytoken endpoint deployment.
///
/// `PATCH /v1/honeytokens/endpoint-deployments/{id}`, answering `200` Deployment updated,
/// `400` Invalid data, `401` Invalid API key, `403` Forbidden Call, `404` Resource not
/// found or `503` API under maintenance.
#[derive(Clone, Debug)]
pub struct ConfirmEndpointDeployment {
    pub id: String,
    /// Outcome reported by the client.
    pub status: DeploymentStatus,
}

impl ConfirmEndpointDeployment {
    pub fn new(id: impl Into<String>, status: DeploymentStatus) -> Self {
        Self { id: id.into(), status }
    }
}

impl ApiCall for ConfirmEndpointDeployment {
    type Output = EndpointDeployment;

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let url = config.endpoint(&format!("honeytokens/endpoint-deployments/{}", self.id))?;
        json_body(
            config.request(Method::PATCH, &url),
            &ConfirmEndpointDeploymentBody { status: self.status.clone() },
        )
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_json(response, StatusCode::OK)
    }
}
