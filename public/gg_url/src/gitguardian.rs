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

/// Parsed as users write an instance: a bare host means `https://`.
impl FromStr for DashboardUrl {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        HttpUrl::parse_assuming_https(s).map(Self)
    }
}

/// Parsed as users write an instance: a bare host means `https://`.
impl FromStr for ApiUrl {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        HttpUrl::parse_assuming_https(s).map(Self)
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
    )]
    fn dashboard_to_api(dashboard: &str, expected: &str) {
        let api = ApiUrl::from(DashboardUrl(dashboard.parse().unwrap()));
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
    )]
    fn api_to_dashboard(api: &str, expected: &str) {
        let dashboard = DashboardUrl::from(ApiUrl(api.parse().unwrap()));
        assert_eq!(dashboard.0.as_str(), expected);
    }
}
