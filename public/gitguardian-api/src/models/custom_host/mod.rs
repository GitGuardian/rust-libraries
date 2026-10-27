use serde::Deserialize;

use crate::models::custom_host::status::CustomHostStatus;

pub mod status;

#[derive(Clone, Debug, Deserialize)]
pub struct DetectorCustomHostConfig {
    pub detector_name: String,
    #[serde(default)]
    pub checker_default_base_url: Option<String>,
    #[serde(default)]
    pub custom_hosts: Vec<CustomHost>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CustomHost {
    pub base_url: String,
    pub is_active: bool,
    pub status: CustomHostStatus,
}
