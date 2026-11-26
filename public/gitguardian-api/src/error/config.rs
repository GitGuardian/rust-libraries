use gg_url::Error as UrlError;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum ConfigError {
    #[error("invalid api url: {0}")]
    ApiUrl(#[from] UrlError),
    #[error("api key must contain only printable ascii characters")]
    ApiKey,
}
