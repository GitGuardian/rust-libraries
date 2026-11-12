//! The host of a URL, connection string, email address or bare host name names.

use std::fmt::Display;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

use url::Url;

use crate::error::Error;
use crate::generic_uri::GenericUri;
use crate::http_url::HttpUrl;

/// A host of a URL, connection string, email address or bare host name names
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Host(pub(crate) url::Host<String>);

impl FromStr for Host {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let input = s.trim();
        let ip = input.strip_prefix('[').and_then(|ip| ip.strip_suffix(']')).unwrap_or(input);
        if let Ok(ip) = ip.split('%').next().unwrap_or(ip).parse::<IpAddr>() {
            return Ok(Self::from(ip));
        }
        if let Ok(uri) = input.parse::<GenericUri>() {
            return url::Host::parse(uri.host().trim_end_matches('.'))
                .map(Self)
                .map_err(Error::Invalid);
        }
        let input = input.strip_prefix("//").unwrap_or(input);
        Ok(HttpUrl::parse_assuming_https(input)?.host())
    }
}

impl From<IpAddr> for Host {
    fn from(ip: IpAddr) -> Self {
        Self(match ip {
            IpAddr::V4(ip) => url::Host::Ipv4(ip),
            IpAddr::V6(ip) => url::Host::Ipv6(ip),
        })
    }
}

impl Display for Host {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Host {
    /// The host of an HTTP `Host:` (`host[:port]`) or `Origin:` (`scheme://host[:port]`)
    /// header; `None` when the value also carries userinfo, a path, a query or a fragment.
    pub fn from_header(value: &str) -> Option<Self> {
        let value = value.trim();
        let url: HttpUrl = if value.contains("://") {
            value.parse().ok()?
        } else {
            format!("http://{value}").parse().ok()?
        };
        let bare = !url.has_userinfo()
            && url.path() == "/"
            && url.0.query().is_none()
            && url.0.fragment().is_none();
        bare.then(|| url.host())
    }

    /// The DNS domain without a leading `www.`; `None` for an IP address or a
    /// single-label host such as `localhost`.
    pub fn domain(&self) -> Option<&str> {
        let url::Host::Domain(domain) = &self.0 else {
            return None;
        };
        let domain = domain.strip_prefix("www.").unwrap_or(domain);
        (domain.contains('.') && domain.parse::<Ipv4Addr>().is_err()).then_some(domain)
    }

    /// Whether this is `localhost` or a loopback IP address.
    pub fn is_loopback(&self) -> bool {
        match &self.0 {
            url::Host::Domain(domain) => domain == "localhost",
            url::Host::Ipv4(ip) => ip.is_loopback(),
            url::Host::Ipv6(ip) => ip.is_loopback(),
        }
    }

    /// Whether credentials must never be sent here: a local name (`localhost`,
    /// `*.localhost`, `*.local`) or an IP address that is not globally routable,
    /// such as loopback, private, link-local (cloud metadata included), CGNAT,
    /// unique-local or an IPv4-mapped IPv6 of one of those.
    pub fn is_private_or_local(&self) -> bool {
        match &self.0 {
            url::Host::Domain(domain) => {
                domain == "localhost"
                    || domain.ends_with(".localhost")
                    || domain.ends_with(".local")
            }
            url::Host::Ipv4(ip) => Self::is_private_ipv4(ip),
            url::Host::Ipv6(ip) => Self::is_private_ipv6(ip),
        }
    }

    /// `http://` on this host and `port`, e.g. a local server's address.
    pub fn http_url(&self, port: u16) -> HttpUrl {
        let mut url = Url::parse("http://localhost").expect("a literal http URL is valid");
        url.set_host(Some(&self.to_string())).expect("a parsed host is a valid host");
        url.set_port(Some(port)).expect("an http URL with a host takes a port");
        HttpUrl(url)
    }

    fn is_private_ipv4(ip: &Ipv4Addr) -> bool {
        let [first, second, third, _] = ip.octets();
        ip.is_loopback()
            || ip.is_private()
            || ip.is_link_local()
            || ip.is_unspecified()
            || ip.is_broadcast()
            || ip.is_documentation()
            || (first == 100 && (second & 0b1100_0000) == 0b0100_0000)
            || (first == 192 && second == 0 && third == 0)
    }

    fn is_private_ipv6(ip: &Ipv6Addr) -> bool {
        if let Some(mapped) = ip.to_ipv4_mapped() {
            return Self::is_private_ipv4(&mapped);
        }
        let first = ip.segments()[0];
        ip.is_loopback()
            || ip.is_unspecified()
            || (first & 0xfe00) == 0xfc00
            || (first & 0xffc0) == 0xfe80
    }
}

#[cfg(test)]
mod tests {
    use yare::parameterized;

    use super::*;

    /// GIVEN a URL, a connection string, an email address, a protocol-relative URL or a bare host
    /// WHEN reading the domain it names
    /// THEN it is lowercased without `www.` or a trailing dot, and an IP or a single label has none
    #[test]
    fn reads_domain() {
        let domain = |input: &str| input.parse::<Host>().ok()?.domain().map(str::to_owned);
        assert_eq!(domain("https://www.Acme.com./docs").as_deref(), Some("acme.com"));
        assert_eq!(
            domain("postgres://u:p{w@DB.corp.example/x").as_deref(),
            Some("db.corp.example")
        );
        assert_eq!(domain("support@Example.ORG").as_deref(), Some("example.org"));
        assert_eq!(domain("//cdn.example.com/x").as_deref(), Some("cdn.example.com"));
        assert_eq!(domain("example.net:443/path").as_deref(), Some("example.net"));
        assert_eq!(domain("localhost"), None);
        assert_eq!(domain("redis://10.0.0.5:6379"), None);
    }

    /// GIVEN a host
    /// WHEN checking whether credentials may be sent to it
    /// THEN local names and non-routable addresses are refused
    #[parameterized(
        metadata = { "169.254.169.254", true },
        private_v4 = { "10.1.2.3", true },
        cgnat = { "100.64.0.1", true },
        mapped_loopback = { "::ffff:127.0.0.1", true },
        unique_local = { "fd00::1", true },
        zoned_link_local = { "fe80::1%eth0", true },
        local_name = { "printer.local", true },
        public_v4 = { "140.82.112.3", false },
        public_v6 = { "[2606:4700:4700::1111]", false },
        public_domain = { "api.github.com", false },
    )]
    fn detects_private_or_local(host: &str, expected: bool) {
        assert_eq!(host.parse::<Host>().unwrap().is_private_or_local(), expected);
    }
}
