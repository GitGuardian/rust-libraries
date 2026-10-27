use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("invalid base uri: {0}")]
    BaseUri(#[from] url::ParseError),
    #[error("api key must contain only printable ascii characters")]
    ApiKey,
}
