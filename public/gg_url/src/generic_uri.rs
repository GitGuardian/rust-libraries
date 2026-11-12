//! A URI of any scheme that names a host, such as a database connection string.

use std::fmt::Display;
use std::str::FromStr;

use url::Url;

use crate::error::Error;

/// A URI with a host and any scheme, e.g. `postgres://user:pass@db.example.com/app`,
/// read for its metadata (scheme, host, userinfo) to display or attribute.
/// To parse a URL satori will connect to, use [`crate::HttpUrl`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericUri(Url);

impl FromStr for GenericUri {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let url = match s.split_once(':') {
            Some((authority, path))
                if !s.contains("://")
                    && authority.contains('@')
                    && !authority.contains('/')
                    && !path.starts_with(|c: char| c.is_ascii_digit()) =>
            {
                Url::parse(&format!("ssh://{authority}/{path}"))
            }
            _ => Url::parse(s),
        }
        .map_err(Error::Invalid)?;
        if url.host_str().is_none_or(str::is_empty) {
            return Err(Error::NoHost);
        }
        Ok(Self(url))
    }
}

impl Display for GenericUri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl GenericUri {
    pub fn scheme(&self) -> &str {
        self.0.scheme()
    }

    /// The host without port or userinfo; IPv6 stays bracketed.
    pub fn host(&self) -> &str {
        self.0.host_str().unwrap_or_default()
    }

    pub fn has_userinfo(&self) -> bool {
        !self.0.username().is_empty() || self.0.password().is_some()
    }

    /// This URI without its `user:password@`, e.g. before logging it.
    pub fn without_userinfo(mut self) -> Self {
        if self.has_userinfo() {
            self.0
                .set_username("")
                .and_then(|()| self.0.set_password(None))
                .expect("a URI holding userinfo can drop it");
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use yare::parameterized;

    use super::*;

    /// GIVEN a connection string whose password holds `{` and `@`
    /// WHEN parsing it
    /// THEN the scheme and the host after the last `@` are read
    #[test]
    fn reads_scheme_and_host() {
        let uri: GenericUri = "postgres://app:p{a@ss@db.example.com:5432/x".parse().unwrap();
        assert_eq!((uri.scheme(), uri.host()), ("postgres", "db.example.com"));
    }

    /// GIVEN a URI without a host
    /// WHEN parsing it
    /// THEN it is refused
    #[test]
    fn refuses_a_uri_without_host() {
        assert_eq!("mailto:a@example.com".parse::<GenericUri>(), Err(Error::NoHost));
    }

    /// GIVEN a URL
    /// WHEN removing its userinfo
    /// THEN username and password are gone and the rest is kept
    #[parameterized(
        user_and_password = { "https://user:pass@example.com/r.git?q=1", "https://example.com/r.git?q=1" },
        user_only = { "https://user@example.com/r.git", "https://example.com/r.git" },
        none = { "https://example.com/r.git", "https://example.com/r.git" },
    )]
    fn strips_userinfo(url: &str, expected: &str) {
        let url = url.parse::<GenericUri>().unwrap().without_userinfo();
        assert!(!url.has_userinfo());
        assert_eq!(url.to_string(), expected);
    }

    /// GIVEN an scp-style git remote
    /// WHEN parsing it
    /// THEN it reads as the equivalent `ssh://` URI
    #[test]
    fn reads_an_scp_style_remote() {
        let uri: GenericUri = "git@github.com:org/repo.git".parse().unwrap();
        assert_eq!((uri.scheme(), uri.host()), ("ssh", "github.com"));
    }
}
