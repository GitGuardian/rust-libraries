use std::str::FromStr;

use crate::error::Error;
use crate::http_url::HttpUrl;

/// A rendered request URL: absolute, or a path on the base URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestUrl {
    Absolute(HttpUrl),
    Relative(String),
}

impl FromStr for RequestUrl {
    type Err = Error;

    /// A single leading `/` marks a path on the base; `//host` is not one.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.starts_with('/') && !s.starts_with("//") {
            Ok(Self::Relative(s.to_owned()))
        } else {
            s.parse().map(Self::Absolute)
        }
    }
}

impl RequestUrl {
    /// The URL to send the request to: a relative path joins `base`.
    pub fn resolve(self, base: Option<&HttpUrl>) -> Result<HttpUrl, Error> {
        match self {
            Self::Absolute(url) => Ok(url),
            Self::Relative(path) => base.ok_or(Error::MissingBase)?.join_path(&path),
        }
    }
}

#[cfg(test)]
mod tests {
    use url::ParseError;
    use yare::parameterized;

    use super::*;

    /// GIVEN a rendered request URL
    /// WHEN resolving it against a base URL
    /// THEN a path joins the base and an absolute URL ignores it
    #[parameterized(
        relative = { "/v1/me?x=1", "https://corp.example/gitlab/v1/me?x=1" },
        absolute = { "https://api.example.com/v1/me", "https://api.example.com/v1/me" },
    )]
    fn resolves_against_base(rendered: &str, expected: &str) {
        let base = "https://corp.example/gitlab".parse::<HttpUrl>().unwrap();
        let url = rendered.parse::<RequestUrl>().unwrap().resolve(Some(&base)).unwrap();
        assert_eq!(url.as_str(), expected);
    }

    /// GIVEN a rendered request URL that can't be resolved
    /// WHEN parsing and resolving it without a base URL
    /// THEN the expected error is returned
    #[parameterized(
        relative_without_base = { "/v1/me", Error::MissingBase },
        scheme_relative = { "//evil.example/x", Error::Invalid(ParseError::RelativeUrlWithoutBase) },
    )]
    fn fails_to_resolve(rendered: &str, expected_err: Error) {
        let url = rendered.parse::<RequestUrl>().and_then(|url| url.resolve(None));
        assert_eq!(url, Err(expected_err));
    }
}
