use std::borrow::Cow;
use std::fmt::Display;
use std::str::FromStr;

use url::{Origin, Url};

use crate::error::Error;
use crate::host::Host;
use crate::placeholder::Template;

const GITGUARDIAN_SAAS_DOMAINS: [&str; 2] = [".gitguardian.com", ".gitguardian.tech"];

/// An http(s) URL with a host.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HttpUrl(pub(crate) Url);

impl FromStr for HttpUrl {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let input = s.trim();
        if input.has_placeholder() {
            return Err(Error::Templated);
        }
        let mut url = Url::parse(input).map_err(Error::Invalid)?;
        if !matches!(url.scheme(), "http" | "https") {
            return Err(Error::NotHttp);
        }
        let host = url
            .host_str()
            .map(|host| host.trim_end_matches('.').to_owned())
            .filter(|host| !host.is_empty())
            .ok_or(Error::NoHost)?;
        url.set_host(Some(&host)).map_err(Error::Invalid)?;
        Ok(Self(url))
    }
}

impl Display for HttpUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl HttpUrl {
    /// Parse a URL or a bare host as users write them in config: without a
    /// scheme, `https://` is assumed, e.g. `gitlab.corp.example:8443`.
    pub fn parse_assuming_https(input: &str) -> Result<Self, Error> {
        let input = input.trim();
        let has_scheme = input.split_once("://").is_some_and(|(scheme, _)| {
            !scheme.is_empty()
                && scheme.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
        });
        if has_scheme { input.parse() } else { format!("https://{input}").parse() }
    }

    /// The URL without its trailing `/`, as config stores it, unless a query
    /// or fragment follows.
    pub fn base(&self) -> &str {
        let url = self.0.as_str();
        if self.0.query().is_some() || self.0.fragment().is_some() {
            url
        } else {
            url.trim_end_matches('/')
        }
    }

    /// The decoded `name=value` pairs of the query.
    pub fn query_pairs(&self) -> impl Iterator<Item = (Cow<'_, str>, Cow<'_, str>)> {
        self.0.query_pairs()
    }

    /// This URL, unless it is plain HTTP: `http://` is refused except to a
    /// loopback host when `allow_loopback` is set, e.g. a local dev instance.
    pub fn require_secure(self, allow_loopback: bool) -> Result<Self, Error> {
        match self.scheme() {
            "http" if allow_loopback && self.host().is_loopback() => Ok(self),
            "http" => Err(Error::PlaintextRefused),
            _ => Ok(self),
        }
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn scheme(&self) -> &str {
        self.0.scheme()
    }

    pub fn host(&self) -> Host {
        Host(self.0.host().expect("an http(s) URL always has a host").to_owned())
    }

    /// The host as written in the URL
    pub fn host_str(&self) -> &str {
        self.0.host_str().unwrap_or_default()
    }

    /// Whether this is a GitGuardian SaaS dashboard or API host, e.g.
    /// `dashboard.gitguardian.com` or `api-22020.review-apps.gitguardian.tech`.
    pub fn is_gitguardian_saas(&self) -> bool {
        self.0.domain().is_some_and(|domain| {
            GITGUARDIAN_SAAS_DOMAINS.iter().any(|saas| domain.ends_with(saas))
                && ["dashboard", "api"].iter().any(|prefix| {
                    domain.strip_prefix(prefix).is_some_and(|rest| rest.starts_with(['.', '-']))
                })
        })
    }

    /// Rewrites the subdomain part for converting between api and dashboard domains of SaaS including review apps
    pub(crate) fn swap_saas_label_prefix(&mut self, from: &str, to: &str) {
        if !self.is_gitguardian_saas() {
            return;
        }
        let Some(rest) = self
            .0
            .domain()
            .and_then(|domain| domain.strip_prefix(from))
            .filter(|rest| rest.starts_with(['.', '-']))
        else {
            return;
        };
        let host = format!("{to}{rest}");
        self.0.set_host(Some(&host)).expect("swapping the label prefix keeps the host valid");
    }

    /// Scheme, host and port: two URLs with the same origin name the same server.
    pub fn origin(&self) -> Origin {
        self.0.origin()
    }

    pub fn path(&self) -> &str {
        self.0.path()
    }

    /// Sets the path of the URL. If one exists it will be overridden.
    pub fn set_path(&mut self, path: &str) {
        self.0.set_path(path);
    }

    /// Resolve a redirect's `Location` against this URL. The target must keep
    /// this URL's origin and carry no credentials.
    pub fn resolve_location(&self, location: &str) -> Result<Self, Error> {
        let target: Self =
            self.0.join(location.trim()).map_err(Error::Invalid)?.as_str().parse()?;
        if target.origin() != self.origin() {
            return Err(Error::CrossOrigin);
        }
        if target.has_userinfo() {
            return Err(Error::HasUserinfo);
        }
        Ok(target)
    }

    /// Append `path`, which may carry a query, to this URL's path. Unlike
    /// [`Url::join`], the base path is kept: `https://host/gitlab` + `/v1/me`
    /// is `https://host/gitlab/v1/me`. The base's query and fragment are dropped.
    pub(crate) fn join_path(&self, path: &str) -> Result<Self, Error> {
        let mut base = self.0.clone();
        base.set_query(None);
        base.set_fragment(None);
        let base = base.as_str().trim_end_matches('/');
        format!("{base}/{}", path.trim().trim_start_matches('/')).parse()
    }

    /// This URL with a fixed API `path` appended, e.g. `api.endpoint("/v1/token")`.
    /// Each segment is percent-encoded, so a `?` or `#` in `path` stays part of
    /// the path: add a query with [`Self::extend_query`].
    pub fn endpoint(&self, path: &str) -> Self {
        let mut url = self.0.clone();
        url.set_query(None);
        url.set_fragment(None);
        url.path_segments_mut()
            .expect("an http(s) URL always has a path")
            .pop_if_empty()
            .extend(path.trim_start_matches('/').split('/'));
        Self(url)
    }

    pub fn has_userinfo(&self) -> bool {
        !self.0.username().is_empty() || self.0.password().is_some()
    }

    /// Append `params` to the query, form-encoded.
    pub fn extend_query<K: AsRef<str>, V: AsRef<str>>(&mut self, params: &[(K, V)]) {
        if !params.is_empty() {
            self.0.query_pairs_mut().extend_pairs(params);
        }
    }

    /// Whether the query carries a parameter named `name`, compared after decoding.
    pub fn has_query_param(&self, name: &str) -> bool {
        self.0.query_pairs().any(|(key, _)| key == name)
    }
}

#[cfg(test)]
mod tests {
    use url::ParseError;
    use yare::parameterized;

    use super::*;

    /// GIVEN a valid HTTP URL
    /// WHEN parsing it as a HttpUrl
    /// THEN parsing succeeds
    #[parameterized(
        http = { "http://example.com" },
        trimmed = { "  https://example.com  " },
        path = { "https://example.com/a/b" },
        query = { "https://example.com?q=1" },
        fragment = { "https://example.com/#frag" },
        port = { "https://example.com:8443" },
        userinfo = { "https://user:pass@example.com" },
        subdomain = { "https://sub.example.com" },
        ipv4 = { "http://127.0.0.1" },
        ipv6 = { "http://[::1]" },
        idn = { "https://ոоո-аѕϲіі.example" },
        percent_encoded = { "https://example.com/a%20b" },
        backslashes = { "http:\\\\example.com\\path" },
        trailing_dot = { "https://example.com./a" },
    )]
    fn parses_valid_url(url: &str) {
        assert!(url.parse::<HttpUrl>().is_ok(), "expected {url:?} to parse");
    }

    /// GIVEN an invalid HTTP URL
    /// WHEN parsing it as a HttpUrl
    /// THEN parsing fails with the expected error
    #[parameterized(
        empty = { "", Error::Invalid(ParseError::RelativeUrlWithoutBase) },
        no_scheme = { "example.com", Error::Invalid(ParseError::RelativeUrlWithoutBase) },
        empty_host = { "http://", Error::Invalid(ParseError::EmptyHost) },
        bad_port = { "http://example.com:99999", Error::Invalid(ParseError::InvalidPort) },
        bad_ipv4 = { "http://256.0.0.1", Error::Invalid(ParseError::InvalidIpv4Address) },
        bad_ipv6 = { "http://[::1", Error::Invalid(ParseError::InvalidIpv6Address) },
        space_in_host = { "http://a b", Error::Invalid(ParseError::IdnaError) },
        ftp = { "ftp://example.com", Error::NotHttp },
        mailto = { "mailto:a@example.com", Error::NotHttp },
        file = { "file:///etc/passwd", Error::NotHttp },
        templated_host = { "https://{region}.example.com", Error::Templated },
        templated_path = { "https://api.example.com/{org}/x", Error::Templated },
        templated_query = { "https://api.example.com/?key={apikey}", Error::Templated },
        only_dot = { "http://./", Error::NoHost },
    )]
    fn fails_on_invalid_url(url: &str, expected_err: Error) {
        assert_eq!(url.parse::<HttpUrl>(), Err(expected_err));
    }

    /// GIVEN a valid HTTP URL
    /// WHEN reading its host
    /// THEN only the host is returned, without port, userinfo or trailing dot
    #[parameterized(
        domain = { "https://example.com/a", "example.com" },
        port = { "https://example.com:8443", "example.com" },
        userinfo = { "https://user:pass@example.com", "example.com" },
        brace_in_userinfo = { "https://user:p{ss@example.com", "example.com" },
        trailing_dot = { "https://example.com./a", "example.com" },
        uppercase = { "https://EXAMPLE.com", "example.com" },
        ipv4 = { "http://127.0.0.1:8080", "127.0.0.1" },
        ipv6 = { "http://[::1]:8080", "[::1]" },
    )]
    fn extracts_host(url: &str, expected: &str) {
        assert_eq!(url.parse::<HttpUrl>().unwrap().host_str(), expected);
    }

    /// GIVEN a URL whose domain ends in `.`
    /// WHEN parsing it as a HttpUrl
    /// THEN the dot is dropped from the URL that gets sent
    #[parameterized(
        one_dot = { "https://example.com./a", "https://example.com/a" },
        many_dots = { "https://example.com../a", "https://example.com/a" },
        with_port = { "https://example.com.:8443", "https://example.com:8443/" },
        ipv4 = { "http://127.0.0.1./", "http://127.0.0.1/" },
        ipv4_many_dots = { "http://127.0.0.1../", "http://127.0.0.1/" },
    )]
    fn strips_trailing_dot(url: &str, expected: &str) {
        assert_eq!(url.parse::<HttpUrl>().unwrap().as_str(), expected);
    }

    /// GIVEN two URLs
    /// WHEN comparing their origins
    /// THEN they match only when scheme, host and port name the same server
    #[parameterized(
        same = { "https://example.com/a", "https://example.com/b?q=1", true },
        case = { "https://EXAMPLE.com", "https://example.com", true },
        default_port = { "https://example.com:443", "https://example.com", true },
        trailing_dot = { "https://example.com.", "https://example.com", true },
        userinfo = { "https://user@example.com", "https://example.com", true },
        ipv6 = { "http://[::1]:8080", "http://[0:0:0:0:0:0:0:1]:8080", true },
        scheme = { "http://example.com", "https://example.com", false },
        port = { "https://example.com:8443", "https://example.com", false },
        subdomain = { "https://api.example.com", "https://example.com", false },
    )]
    fn compares_origins(a: &str, b: &str, same: bool) {
        let (a, b) = (a.parse::<HttpUrl>().unwrap(), b.parse::<HttpUrl>().unwrap());
        assert_eq!(a.origin() == b.origin(), same);
    }

    /// GIVEN a valid HTTP URL
    /// WHEN checking whether it is loopback
    /// THEN only `localhost` and loopback IPs are reported as such
    #[parameterized(
        localhost = { "http://localhost", true },
        localhost_port = { "http://localhost:8080/a", true },
        localhost_trailing_dot = { "http://localhost./", true },
        ipv4_many_trailing_dots = { "http://127.0.0.1../", true },
        localhost_uppercase = { "http://LOCALHOST", true },
        ipv4 = { "http://127.0.0.1", true },
        ipv4_other_block = { "http://127.1.2.3", true },
        ipv6 = { "http://[::1]", true },
        domain = { "https://example.com", false },
        localhost_subdomain = { "https://localhost.example.com", false },
        localhost_in_userinfo = { "https://localhost@example.com", false },
        private_ipv4 = { "http://192.168.0.1", false },
        unspecified_ipv4 = { "http://0.0.0.0", false },
        public_ipv6 = { "http://[2001:db8::1]", false },
    )]
    fn detects_loopback(url: &str, expected: bool) {
        assert_eq!(url.parse::<HttpUrl>().unwrap().host().is_loopback(), expected);
    }

    /// GIVEN a plaintext HTTP URL
    /// WHEN requiring it to be secure
    /// THEN it is only accepted if it is loopback and loopback is allowed
    #[parameterized(
        loopback_allowed = { "http://localhost:8080", true, false },
        loopback_not_allowed = { "http://localhost:8080", false, true },
        remote_with_loopback_allowed = { "http://example.com", true, true },
    )]
    fn require_secure_refuses_plaintext(url: &str, allow_loopback: bool, is_err: bool) {
        let url: HttpUrl = url.parse().unwrap();
        assert_eq!(url.require_secure(allow_loopback).is_err(), is_err);
    }

    /// GIVEN a same-origin `Location`
    /// WHEN resolving it against the request URL
    /// THEN the full target URL is returned, query included
    #[parameterized(
        absolute_path = { "/v2/me", "https://api.example.com/v2/me" },
        relative_path = { "me", "https://api.example.com/v1/me" },
        dot_segments = { "../v2/me", "https://api.example.com/v2/me" },
        query = { "/v2/me?page=2", "https://api.example.com/v2/me?page=2" },
        absolute_url = { "https://api.example.com/v2", "https://api.example.com/v2" },
        explicit_default_port = { "https://api.example.com:443/v2", "https://api.example.com/v2" },
        scheme_relative = { "//api.example.com/v2", "https://api.example.com/v2" },
        trailing_dot = { "https://api.example.com./v2", "https://api.example.com/v2" },
        trimmed = { "  /v2/me  ", "https://api.example.com/v2/me" },
    )]
    fn resolves_same_origin_location(location: &str, expected: &str) {
        let from = "https://api.example.com/v1/users?page=1".parse::<HttpUrl>().unwrap();
        assert_eq!(from.resolve_location(location).unwrap().as_str(), expected);
    }

    /// GIVEN a `Location` that leaves the origin or carries credentials
    /// WHEN resolving it against the request URL
    /// THEN resolving fails with the expected error
    #[parameterized(
        other_host = { "https://evil.example/v2", Error::CrossOrigin },
        scheme_relative_other_host = { "//evil.example/v2", Error::CrossOrigin },
        subdomain = { "https://sub.api.example.com/v2", Error::CrossOrigin },
        downgrade = { "http://api.example.com/v2", Error::CrossOrigin },
        other_port = { "https://api.example.com:8443/v2", Error::CrossOrigin },
        other_scheme = { "ftp://api.example.com/v2", Error::NotHttp },
        templated = { "/v2/me?q={x}", Error::Templated },
        userinfo = { "https://user:pass@api.example.com/v2", Error::HasUserinfo },
        username_only = { "https://user@api.example.com/v2", Error::HasUserinfo },
        bad_port = { "https://api.example.com:99999/", Error::Invalid(ParseError::InvalidPort) },
    )]
    fn refuses_unsafe_location(location: &str, expected_err: Error) {
        let from = "https://api.example.com/v1/users".parse::<HttpUrl>().unwrap();
        assert_eq!(from.resolve_location(location), Err(expected_err));
    }

    /// GIVEN a URL and query parameters
    /// WHEN extending its query
    /// THEN the parameters are appended form-encoded, keeping the existing query and fragment
    #[parameterized(
        no_query = { "https://example.com/a", &[("k", "v")], "https://example.com/a?k=v" },
        existing_query = { "https://example.com/a?x=1", &[("k", "v")], "https://example.com/a?x=1&k=v" },
        empty_query = { "https://example.com/a?", &[("k", "v")], "https://example.com/a?k=v" },
        several = { "https://example.com", &[("a", "1"), ("b", "2")], "https://example.com/?a=1&b=2" },
        fragment = { "https://example.com/a#f", &[("k", "v")], "https://example.com/a?k=v#f" },
        reserved = { "https://example.com", &[("q", "a b&c=d")], "https://example.com/?q=a+b%26c%3Dd" },
        non_ascii = { "https://example.com", &[("name", "é")], "https://example.com/?name=%C3%A9" },
        no_params = { "https://example.com/a", &[], "https://example.com/a" },
    )]
    fn extends_query(url: &str, params: &[(&str, &str)], expected: &str) {
        let mut url = url.parse::<HttpUrl>().unwrap();
        url.extend_query(params);
        assert_eq!(url.as_str(), expected);
    }

    /// GIVEN a URL with a query
    /// WHEN checking for a parameter name
    /// THEN only an exact, case-sensitive match of a decoded name is found
    #[parameterized(
        present = { "https://example.com/?api_key=1", true },
        among_others = { "https://example.com/?a=1&api_key", true },
        percent_encoded = { "https://example.com/?api%5Fkey=1", true },
        other_case = { "https://example.com/?API_KEY=1", false },
        prefix = { "https://example.com/?api_key_id=1", false },
        as_value = { "https://example.com/?key=api_key", false },
        no_query = { "https://example.com/", false },
    )]
    fn finds_query_param(url: &str, expected: bool) {
        assert_eq!(url.parse::<HttpUrl>().unwrap().has_query_param("api_key"), expected);
    }

    /// GIVEN a base URL and a path
    /// WHEN joining them
    /// THEN the path is appended to the base path
    #[parameterized(
        no_base_path = { "https://gitlab.com", "/v1/me", "https://gitlab.com/v1/me" },
        base_path = { "https://corp.example/gitlab/", "/v1/me", "https://corp.example/gitlab/v1/me" },
        path_query = { "https://gitlab.com", "/v1/me?per_page=1", "https://gitlab.com/v1/me?per_page=1" },
        base_query = { "https://gitlab.com/?x=1#f", "/v1/me", "https://gitlab.com/v1/me" },
        authority_like = { "https://gitlab.com", "//evil.example/x", "https://gitlab.com/evil.example/x" },
    )]
    fn joins_path(base: &str, path: &str, expected: &str) {
        let joined = base.parse::<HttpUrl>().unwrap().join_path(path).unwrap();
        assert_eq!(joined.as_str(), expected);
    }

    /// GIVEN an API base URL and a fixed path
    /// WHEN building the endpoint
    /// THEN the path is appended to the base path, dropping the base's query
    #[parameterized(
        no_base_path = { "https://api.example.com", "/v1/token", "https://api.example.com/v1/token" },
        base_path = { "https://gg.example/exposed/", "/v1/token", "https://gg.example/exposed/v1/token" },
        base_query = { "https://api.example.com/?x=1", "/v1/token", "https://api.example.com/v1/token" },
    )]
    fn builds_endpoint(base: &str, path: &str, expected: &str) {
        let endpoint = base.parse::<HttpUrl>().unwrap().endpoint(path);
        assert_eq!(endpoint.as_str(), expected);
    }

    /// GIVEN a URL or a bare host
    /// WHEN parsing it assuming https
    /// THEN a missing scheme becomes `https://` and an existing one is kept
    #[parameterized(
        bare_host = { "gitlab.corp.example", "https://gitlab.corp.example/" },
        host_port = { "localhost:3000", "https://localhost:3000/" },
        http_kept = { "http://localhost:3000", "http://localhost:3000/" },
        scheme_in_query = { "host.example/r?to=http://x", "https://host.example/r?to=http://x" },
    )]
    fn parses_assuming_https(input: &str, expected: &str) {
        assert_eq!(HttpUrl::parse_assuming_https(input).unwrap().as_str(), expected);
    }

    /// GIVEN a URL
    /// WHEN reading its base
    /// THEN the trailing `/` is dropped unless a query or fragment follows
    #[parameterized(
        root = { "https://example.com/", "https://example.com" },
        path = { "https://example.com/exposed/", "https://example.com/exposed" },
        query = { "https://example.com/?x=1", "https://example.com/?x=1" },
    )]
    fn returns_base(url: &str, expected: &str) {
        assert_eq!(url.parse::<HttpUrl>().unwrap().base(), expected);
    }
}
