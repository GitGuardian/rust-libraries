use serde::Deserialize;

/// Short lived JWT for authentication to other GitGuardian services.
#[derive(Clone, Debug, Deserialize)]
pub struct Jwt {
    pub token: String,
}
