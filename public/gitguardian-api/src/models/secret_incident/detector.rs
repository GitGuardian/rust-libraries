use serde::Deserialize;

/// Detector that flagged a secret incident.
#[derive(Clone, Debug, Deserialize)]
pub struct Detector {
    pub name: String,
    pub display_name: String,
    pub nature: DetectorNature,
    #[serde(default)]
    pub family: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub detector_group_name: Option<String>,
    #[serde(default)]
    pub detector_group_display_name: Option<String>,
}

/// Nature of a detector.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DetectorNature {
    Specific,
    Generic,
    Custom,
}
