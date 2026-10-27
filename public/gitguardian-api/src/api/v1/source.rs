use bytes::Bytes;
use http::{Method, Request, Response, StatusCode};
use serde::Serialize;

use crate::error::{ApiError, BuildError};
use crate::models::page::Page;
use crate::models::pagination::{Cursor, Pagination};
use crate::models::source::Source;
use crate::models::source::criticality::SourceCriticality;
use crate::models::source::health::SourceHealth;
use crate::models::source::ordering::SourceOrdering;
use crate::models::source::scan_status::ScanStatus;
use crate::models::source::r#type::SourceType;
use crate::models::source::visibility::SourceVisibility;
use crate::util::json::expect_json_page;
use crate::util::url::set_query;
use crate::{ApiCall, ApiConfig, Paginated};

/// Query parameters of a source listing request
#[derive(Clone, Debug, Serialize)]
struct ListSourcesQueryParam<'a> {
    #[serde(flatten)]
    page: &'a Pagination,
    #[serde(skip_serializing_if = "Option::is_none")]
    search: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_scan_status: Option<ScanStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    health: Option<SourceHealth>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    source_type: Option<SourceType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ordering: Option<SourceOrdering>,
    #[serde(skip_serializing_if = "Option::is_none")]
    visibility: Option<SourceVisibility>,
    #[serde(skip_serializing_if = "Option::is_none")]
    external_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_criticality: Option<SourceCriticality>,
    #[serde(skip_serializing_if = "Option::is_none")]
    monitored: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    provider_metadata_archived: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    team_id: Option<u32>,
    #[serde(rename = "monitoring_status__in", skip_serializing_if = "Option::is_none")]
    monitoring_status_in: Option<&'a str>,
    #[serde(rename = "monitoring_status__nin", skip_serializing_if = "Option::is_none")]
    monitoring_status_nin: Option<&'a str>,
}

/// List sources.
///
/// List sources known by GitGuardian.
///
/// `GET /v1/sources`, answering `200` List sources, `400` Invalid data, `401` Invalid API
/// key or `503` API under maintenance.
#[derive(Clone, Debug, Default)]
pub struct ListSources {
    pub page: Pagination,
    /// Sources matching this search.
    pub search: Option<String>,
    /// Filter sources based on the status of their latest historical scan.
    pub last_scan_status: Option<ScanStatus>,
    /// Filter sources based on their health status.
    pub health: Option<SourceHealth>,
    /// Filter by source type.
    pub source_type: Option<SourceType>,
    /// Sort the results by their field value. The default sort is ASC, DESC if the field
    /// is preceded by a `-`.
    pub ordering: Option<SourceOrdering>,
    /// Filter by visibility status.
    pub visibility: Option<SourceVisibility>,
    /// Filter by specific external id.
    pub external_id: Option<String>,
    /// Filter by source criticality.
    pub source_criticality: Option<SourceCriticality>,
    /// Filter by monitored value.
    pub monitored: Option<bool>,
    /// Filter by whether the source is archived on the provider side when that information
    /// is available.
    pub provider_metadata_archived: Option<bool>,
    /// Filter by the id of a team. Only sources belonging to the given team's perimeter are
    /// returned.
    pub team_id: Option<u32>,
    /// Keep only sources whose real-time monitoring status is one of the given
    /// (comma-separated) buckets.
    pub monitoring_status_in: Option<String>,
    /// Exclude sources whose real-time monitoring status is one of the given
    /// (comma-separated) buckets.
    pub monitoring_status_nin: Option<String>,
}

impl ApiCall for ListSources {
    type Output = Page<Source>;

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let mut url = config.endpoint("sources")?;
        set_query(
            &mut url,
            &ListSourcesQueryParam {
                page: &self.page,
                search: self.search.as_deref(),
                last_scan_status: self.last_scan_status.clone(),
                health: self.health.clone(),
                source_type: self.source_type,
                ordering: self.ordering,
                visibility: self.visibility,
                external_id: self.external_id.as_deref(),
                source_criticality: self.source_criticality,
                monitored: self.monitored,
                provider_metadata_archived: self.provider_metadata_archived,
                team_id: self.team_id,
                monitoring_status_in: self.monitoring_status_in.as_deref(),
                monitoring_status_nin: self.monitoring_status_nin.as_deref(),
            },
        )?;
        Ok(config.request(Method::GET, &url).body(Bytes::new())?)
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_json_page(response, StatusCode::OK)
    }
}

impl Paginated for ListSources {
    fn set_cursor(&mut self, cursor: Cursor) {
        self.page.cursor = Some(cursor);
    }
}
