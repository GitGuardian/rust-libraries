#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("invalid URL: {0}")]
    Invalid(url::ParseError),
    #[error("not an http(s) URL")]
    NotHttp,
    #[error("URL has no host")]
    NoHost,
    #[error("URL still holds a {{placeholder}}")]
    Templated,
    #[error("relative URL without a base URL")]
    MissingBase,
    #[error("refusing plaintext HTTP to a non-loopback host")]
    PlaintextRefused,
    #[error("redirect leaves the origin")]
    CrossOrigin,
    #[error("URL carries credentials")]
    HasUserinfo,
}
