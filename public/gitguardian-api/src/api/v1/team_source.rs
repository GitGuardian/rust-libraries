use bytes::Bytes;
use http::{Method, Request, Response, StatusCode};
use serde::Serialize;

use crate::error::{ApiError, BuildError};
use crate::models::page::Page;
use crate::models::pagination::{Cursor, Pagination};
use crate::models::source::Source;
use crate::models::source::health::SourceHealth;
use crate::models::source::ordering::SourceOrdering;
use crate::models::source::scan_status::ScanStatus;
use crate::models::source::r#type::SourceType;
use crate::models::source::visibility::SourceVisibility;
use crate::util::json::{expect_json_page, expect_no_content, json_body};
use crate::util::url::set_query;
use crate::{ApiCall, ApiConfig, Paginated};

/// Body of a team perimeter update request
#[derive(Clone, Debug, Serialize)]
struct UpdateTeamSourcesBody<'a> {
    sources_to_add: &'a [u32],
    sources_to_remove: &'a [u32],
}

/// Update a team perimeter.
///
/// This endpoint allows you to add and remove sources from the perimeter of a team.
///
/// If you are using a personal access token, you need to be a workspace manager.
///
/// `POST /v1/teams/{team_id}/sources`, answering `204` Team perimeter updated, `400`
/// Invalid data, `401` Invalid API key, `403` Permission denied, `404` Not found or `503`
/// API under maintenance.
#[derive(Clone, Debug)]
pub struct UpdateTeamSources {
    /// The id of the team.
    pub team_id: u32,
    /// Ids of sources to add to the perimeter.
    pub sources_to_add: Vec<u32>,
    /// Ids of sources to remove from the perimeter.
    pub sources_to_remove: Vec<u32>,
}

impl UpdateTeamSources {
    pub fn new(team_id: u32) -> Self {
        Self { team_id, sources_to_add: Vec::new(), sources_to_remove: Vec::new() }
    }
}

impl ApiCall for UpdateTeamSources {
    type Output = ();

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let url = config.endpoint(&format!("teams/{}/sources", self.team_id))?;
        json_body(
            config.request(Method::POST, &url),
            &UpdateTeamSourcesBody {
                sources_to_add: &self.sources_to_add,
                sources_to_remove: &self.sources_to_remove,
            },
        )
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_no_content(response, StatusCode::NO_CONTENT)
    }
}

/// Query parameters of a team source listing request
#[derive(Clone, Debug, Serialize)]
struct ListTeamSourcesQueryParam<'a> {
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
}

/// List team sources.
///
/// List sources belonging to a team's perimeter.
///
/// `GET /v1/teams/{team_id}/sources`, answering `200` List team sources, `400` Invalid
/// data, `401` Invalid API key, `403` Permission denied, `404` Not found or `503` API under
/// maintenance.
#[derive(Clone, Debug)]
pub struct ListTeamSources {
    /// The id of the team.
    pub team_id: u32,
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
}

impl ListTeamSources {
    pub fn new(team_id: u32) -> Self {
        Self {
            team_id,
            page: Pagination::default(),
            search: None,
            last_scan_status: None,
            health: None,
            source_type: None,
            ordering: None,
            visibility: None,
            external_id: None,
        }
    }
}

impl ApiCall for ListTeamSources {
    type Output = Page<Source>;

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let mut url = config.endpoint(&format!("teams/{}/sources", self.team_id))?;
        set_query(
            &mut url,
            &ListTeamSourcesQueryParam {
                page: &self.page,
                search: self.search.as_deref(),
                last_scan_status: self.last_scan_status.clone(),
                health: self.health.clone(),
                source_type: self.source_type,
                ordering: self.ordering,
                visibility: self.visibility,
                external_id: self.external_id.as_deref(),
            },
        )?;
        Ok(config.request(Method::GET, &url).body(Bytes::new())?)
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_json_page(response, StatusCode::OK)
    }
}

impl Paginated for ListTeamSources {
    fn set_cursor(&mut self, cursor: Cursor) {
        self.page.cursor = Some(cursor);
    }
}
