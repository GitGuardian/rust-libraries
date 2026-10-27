use serde::Deserialize;

/// Public exposure of a secret incident.
#[derive(Clone, Debug, Deserialize)]
pub struct PublicExposure {
    pub source_publicly_visible: bool,
    pub public_incident_linked: bool,
    pub leaked_outside_perimeter: bool,
}
