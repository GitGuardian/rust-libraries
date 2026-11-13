use serde::Deserialize;

/// Short lived JWT for authentication to other GitGuardian services.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Jwt {
    pub token: String,
}
