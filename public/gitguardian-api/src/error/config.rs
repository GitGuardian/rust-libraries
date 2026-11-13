use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum ConfigError {
    #[error("invalid base uri: {0}")]
    BaseUri(#[from] url::ParseError),
    #[error("api key must contain only printable ascii characters")]
    ApiKey,
}
