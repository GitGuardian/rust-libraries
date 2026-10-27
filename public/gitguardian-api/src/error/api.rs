use http::StatusCode;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("{status}: {detail}")]
    Status { status: StatusCode, detail: String },
    #[error("failed to deserialize {status} response: {source}")]
    Deserialize {
        status: StatusCode,
        #[source]
        source: serde_json::Error,
    },
}
