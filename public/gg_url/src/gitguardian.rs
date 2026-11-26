//! The two URLs of a GitGuardian instance: the dashboard and its API.

use std::fmt::Display;
use std::str::FromStr;

use crate::error::Error;
use crate::http_url::HttpUrl;

/// A GitGuardian dashboard URL, e.g. `https://dashboard.gitguardian.com`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DashboardUrl(pub HttpUrl);

/// A GitGuardian API URL, e.g. `https://api.gitguardian.com`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ApiUrl(pub HttpUrl);

/// Parsed as users write an instance: a bare host means `https://` and a
/// trailing `/v1` is removed. Plain HTTP is accepted only for a loopback host.
impl FromStr for DashboardUrl {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut url = HttpUrl::parse_assuming_https(s)?.require_secure(true)?;
        url.remove_path_suffix("/v1");
        Ok(Self(url))
    }
}

/// Parsed as users write an API URL: a bare host means `https://` and a
/// trailing `/v1` is removed. Plain HTTP is accepted only for a loopback host.
impl FromStr for ApiUrl {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut url = HttpUrl::parse_assuming_https(s)?.require_secure(true)?;
        url.remove_path_suffix("/v1");
        Ok(Self(url))
    }
}

/// GitGuardian SaaS: `https://dashboard.gitguardian.com`.
impl Default for DashboardUrl {
    fn default() -> Self {
        "https://dashboard.gitguardian.com".parse().expect("the SaaS dashboard URL is valid")
    }
}

/// GitGuardian SaaS: `https://api.gitguardian.com`.
impl Default for ApiUrl {
    fn default() -> Self {
        Self::from(DashboardUrl::default())
    }
}

/// Without a trailing `/`, as config stores it.
impl Display for DashboardUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.base())
    }
}

/// Without a trailing `/`, as config stores it.
impl Display for ApiUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.base())
    }
}

impl ApiUrl {
    /// `path` on this API, which receives a token: plain HTTP is refused unless
    /// the instance is on a loopback host, e.g. a local dev instance.
    pub fn secure_endpoint(&self, path: &str) -> Result<HttpUrl, Error> {
        self.0.clone().require_secure(true).map(|url| url.endpoint(path))
    }
}

/// SaaS `dashboard[…]` serves its API at `api[…]`; self-hosted under `/exposed`.
/// Converting a URL that already is the API URL leaves it unchanged.
impl From<DashboardUrl> for ApiUrl {
    fn from(DashboardUrl(mut url): DashboardUrl) -> Self {
        if url.is_gitguardian_saas() {
            url.swap_saas_label_prefix("dashboard", "api");
        } else {
            let path = url.path().trim_end_matches('/');
            let path = path.strip_suffix("/exposed").unwrap_or(path);
            let path = format!("{path}/exposed");
            url.set_path(&path);
        }
        Self(url)
    }
}

impl From<ApiUrl> for DashboardUrl {
    fn from(ApiUrl(mut url): ApiUrl) -> Self {
        if url.is_gitguardian_saas() {
            url.swap_saas_label_prefix("api", "dashboard");
        } else {
            let path = url.path().trim_end_matches('/');
            let path = path.strip_suffix("/exposed").unwrap_or(path).to_owned();
            url.set_path(&path);
        }
        Self(url)
    }
}

#[cfg(test)]
mod tests {
    use yare::parameterized;

    use super::*;

    /// GIVEN a GitGuardian URL as users write it
    /// WHEN parsing it as an API URL or as a dashboard URL
    /// THEN one trailing `/v1` path segment is removed, and nothing else
    #[parameterized(
        saas = { "https://api.gitguardian.com/v1", "https://api.gitguardian.com" },
        trailing_slash = { "https://api.gitguardian.com/v1/", "https://api.gitguardian.com" },
        after_exposed = { "https://gg.example.com/exposed/v1", "https://gg.example.com/exposed" },
        self_hosted = { "https://gg.example.com/v1", "https://gg.example.com" },
        part_of_a_segment = { "https://gg.example.com/apiv1", "https://gg.example.com/apiv1" },
        not_at_the_end = { "https://gg.example.com/v1/exposed", "https://gg.example.com/v1/exposed" },
        twice = { "https://gg.example.com/v1/v1", "https://gg.example.com/v1" },
        before_a_query = { "https://gg.example.com/v1?x=1", "https://gg.example.com/?x=1" },
        before_a_fragment = { "https://gg.example.com/v1#f", "https://gg.example.com/#f" },
        inside_a_query = { "https://gg.example.com/?next=/v1", "https://gg.example.com/?next=/v1" },
    )]
    fn removes_trailing_v1(input: &str, expected: &str) {
        assert_eq!(input.parse::<ApiUrl>().unwrap().to_string(), expected);
        assert_eq!(input.parse::<DashboardUrl>().unwrap().to_string(), expected);
    }

    /// GIVEN a plain HTTP URL
    /// WHEN parsing it as an API URL or as a dashboard URL
    /// THEN it is refused unless its host is loopback
    #[parameterized(
        remote = { "http://gg.example.com", Err(Error::PlaintextRefused) },
        loopback = { "http://localhost:3000", Ok(()) },
    )]
    fn refuses_plain_http(input: &str, expected: Result<(), Error>) {
        assert_eq!(input.parse::<ApiUrl>().map(drop), expected);
        assert_eq!(input.parse::<DashboardUrl>().map(drop), expected);
    }

    /// GIVEN a dashboard URL
    /// WHEN converting it to its API URL
    /// THEN SaaS swaps `dashboard` for `api` and self-hosted appends `/exposed`
    #[parameterized(
        saas = { "https://dashboard.gitguardian.com", "https://api.gitguardian.com/" },
        review_app = {
            "https://dashboard-22020.review-apps.gitguardian.tech",
            "https://api-22020.review-apps.gitguardian.tech/"
        },
        self_hosted = { "https://gg.corp.example/", "https://gg.corp.example/exposed" },
        lookalike_domain = {
            "https://dashboard.gitguardian.com.evil.io",
            "https://dashboard.gitguardian.com.evil.io/exposed"
        },
        lookalike_label = {
            "https://dashboards.gitguardian.com",
            "https://dashboards.gitguardian.com/exposed"
        },
        already_api = { "https://api.gitguardian.com", "https://api.gitguardian.com/" },
        self_hosted_with_v1 = { "https://gg.example.com/v1", "https://gg.example.com/exposed" },
    )]
    fn dashboard_to_api(dashboard: &str, expected: &str) {
        let api = ApiUrl::from(dashboard.parse::<DashboardUrl>().unwrap());
        assert_eq!(api.0.as_str(), expected);
    }

    /// GIVEN an API URL
    /// WHEN converting it to its dashboard URL
    /// THEN SaaS swaps `api` for `dashboard` and self-hosted drops `/exposed`
    #[parameterized(
        saas = { "https://api.gitguardian.com", "https://dashboard.gitguardian.com/" },
        api_inside_label = {
            "https://api-apiteam.review-apps.gitguardian.tech",
            "https://dashboard-apiteam.review-apps.gitguardian.tech/"
        },
        self_hosted = { "https://gg.corp.example/exposed/", "https://gg.corp.example/" },
        without_exposed = { "https://gg.corp.example/api", "https://gg.corp.example/api" },
        saas_with_v1 = { "https://api.gitguardian.com/v1", "https://dashboard.gitguardian.com/" },
        self_hosted_with_v1 = { "https://gg.example.com/exposed/v1", "https://gg.example.com/" },
    )]
    fn api_to_dashboard(api: &str, expected: &str) {
        let dashboard = DashboardUrl::from(api.parse::<ApiUrl>().unwrap());
        assert_eq!(dashboard.0.as_str(), expected);
    }
}
