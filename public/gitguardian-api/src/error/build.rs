use thiserror::Error;

#[derive(Debug, Error)]
pub enum BuildError {
    #[error("invalid uri: {0}")]
    Uri(#[from] url::ParseError),
    #[error(transparent)]
    Http(#[from] http::Error),
    #[error("failed to serialize request body: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("failed to serialize query parameters: {0}")]
    Query(#[from] serde_urlencoded::ser::Error),
}
