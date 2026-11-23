use gg_url::DashboardUrl;

use crate::{Location, TokenInfo};

/// A stored token, without its secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredToken {
    /// The dashboard URL of the instance.
    pub instance: DashboardUrl,
    /// The name of the instance, if it has one.
    pub instance_name: Option<String>,
    /// The details of the token.
    pub info: TokenInfo,
    /// Where the token is stored.
    pub location: Location,
}
