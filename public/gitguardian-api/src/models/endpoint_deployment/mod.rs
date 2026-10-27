pub mod action;
pub mod config;
pub mod method;
pub mod status;
pub mod token;

use serde::Deserialize;

use crate::models::endpoint_deployment::action::DeploymentAction;
use crate::models::endpoint_deployment::config::DeploymentConfig;
use crate::models::endpoint_deployment::method::DeploymentMethod;
use crate::models::endpoint_deployment::status::DeploymentStatus;
use crate::models::endpoint_deployment::token::DeploymentToken;
use crate::uuid::Uuid;

/// Desired honeytoken placements for a machine/user.
#[derive(Clone, Debug, Deserialize)]
pub struct EndpointDeployments {
    #[serde(default)]
    pub deployments: Vec<EndpointDeployment>,
}

/// One desired honeytoken placement returned by the endpoint-deployments POST/GET.
#[derive(Clone, Debug, Deserialize)]
pub struct EndpointDeployment {
    /// Deployment id (use it for the status-update PATCH).
    pub id: Uuid,
    #[serde(rename = "type", default)]
    pub deployment_type: Option<String>,
    /// Placement method (sibling discriminator) — drives how the client materializes
    /// `config` (which file + section format).
    pub method: DeploymentMethod,
    #[serde(default)]
    pub config: Option<DeploymentConfig>,
    /// Client-reported outcome.
    #[serde(default)]
    pub status: Option<DeploymentStatus>,
    #[serde(default)]
    pub action: Option<DeploymentAction>,
    #[serde(default)]
    pub token: Option<DeploymentToken>,
}
