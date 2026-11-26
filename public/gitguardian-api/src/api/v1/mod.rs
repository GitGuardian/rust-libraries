pub mod api_token;
pub mod custom_host;
pub mod endpoint_deployment;
pub mod health;
pub mod honeytoken;
pub mod honeytoken_prefix;
pub mod honeytoken_with_context;
pub mod invitation;
pub mod jwt;
pub mod member;
pub mod metadata;
pub mod multiscan;
pub mod oauth;
pub mod quota;
pub mod scan;
pub mod scan_create_incidents;
pub mod secret_incident;
pub mod source;
pub mod team;
pub mod team_invitation;
pub mod team_membership;
pub mod team_source;

use url::Url;

use crate::ApiConfig;
use crate::error::BuildError;

impl ApiConfig {
    pub(crate) fn v1_endpoint(&self, path: &str) -> Result<Url, BuildError> {
        if self.base.path().ends_with("/v1/") {
            return self.endpoint(path);
        }
        self.endpoint(&format!("v1/{path}"))
    }
}
