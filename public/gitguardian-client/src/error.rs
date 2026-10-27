use gitguardian_api::error::{ApiError, BuildError};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Build(#[from] BuildError),
    #[error(transparent)]
    Api(#[from] ApiError),
    #[cfg(feature = "reqwest")]
    #[error(transparent)]
    Reqwest(#[from] ::reqwest::Error),
    #[cfg(feature = "reqwest")]
    #[error(transparent)]
    Transport(Box<dyn std::error::Error + Send + Sync>),
    #[cfg(feature = "ureq")]
    #[error(transparent)]
    Ureq(#[from] ::ureq::Error),
}
