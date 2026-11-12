//! `application/x-www-form-urlencoded` text: URL queries and form bodies.

use std::borrow::Cow;

/// A form-encoded query string or form body, e.g. `code=x&state=y`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Query<'a>(&'a str);

impl<'a> From<&'a str> for Query<'a> {
    fn from(query: &'a str) -> Self {
        Self(query)
    }
}

impl<'a> Query<'a> {
    /// The query of an HTTP request target such as `/callback?code=x&state=y`.
    pub fn of_request_target(target: &'a str) -> Self {
        Self(
            target
                .split_once('?')
                .map_or("", |(_, query)| query.split('#').next().unwrap_or_default()),
        )
    }

    /// The decoded `name=value` pairs, `+` as a space.
    pub fn pairs(self) -> impl Iterator<Item = (Cow<'a, str>, Cow<'a, str>)> {
        url::form_urlencoded::parse(self.0.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// GIVEN an HTTP request target
    /// WHEN reading its query
    /// THEN the pairs are decoded, `+` as a space
    #[test]
    fn reads_the_query_of_a_request_target() {
        let pairs: Vec<_> = Query::of_request_target("/?code=a%2Fb&state=x+y").pairs().collect();
        assert_eq!(pairs, [("code".into(), "a/b".into()), ("state".into(), "x y".into())]);
    }
}
