use bytes::Bytes;
use http::{Method, Request, Response, StatusCode};
use serde::Serialize;

use crate::error::{ApiError, BuildError};
use crate::models::page::Page;
use crate::models::pagination::{Cursor, Pagination};
use crate::models::team::Team;
use crate::util::json::{expect_json, expect_json_page, expect_no_content, json_body};
use crate::util::url::set_query;
use crate::{ApiCall, ApiConfig, Paginated};

/// Body of a team creation request
#[derive(Clone, Debug, Serialize)]
struct CreateTeamBody<'a> {
    name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
}

/// Create a team.
///
/// This endpoint allows you to create a team.
///
/// If you are using a personal access token, you need to have an access level superior or
/// equal to `manager`, and the member is automatically added to the created team with
/// permissions `can_manage` and `full_access`.
///
/// `POST /v1/teams`, answering `201` Team created, `400` Invalid data, `401` Invalid API
/// key, `403` Forbidden Call, `409` Data conflict or `503` API under maintenance.
#[derive(Clone, Debug)]
pub struct CreateTeam {
    pub name: String,
    /// Team description.
    pub description: Option<String>,
}

impl CreateTeam {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), description: None }
    }
}

impl ApiCall for CreateTeam {
    type Output = Team;

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let url = config.endpoint("teams")?;
        json_body(
            config.request(Method::POST, &url),
            &CreateTeamBody { name: &self.name, description: self.description.as_deref() },
        )
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_json(response, StatusCode::CREATED)
    }
}

#[derive(Clone, Debug, Serialize)]
struct ListTeamsQueryParam<'a> {
    #[serde(flatten)]
    page: &'a Pagination,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_global: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    search: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    linked_to_an_external_provider: Option<bool>,
}

#[derive(Clone, Debug, Default)]
pub struct ListTeams {
    pub page: Pagination,
    pub is_global: Option<bool>,
    pub search: Option<String>,
    pub linked_to_an_external_provider: Option<bool>,
}

impl ApiCall for ListTeams {
    type Output = Page<Team>;

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let mut url = config.endpoint("teams")?;
        set_query(
            &mut url,
            &ListTeamsQueryParam {
                page: &self.page,
                is_global: self.is_global,
                search: self.search.as_deref(),
                linked_to_an_external_provider: self.linked_to_an_external_provider,
            },
        )?;
        Ok(config.request(Method::GET, &url).body(Bytes::new())?)
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_json_page(response, StatusCode::OK)
    }
}

impl Paginated for ListTeams {
    fn set_cursor(&mut self, cursor: Cursor) {
        self.page.cursor = Some(cursor);
    }
}

/// Retrieve a team.
///
/// Retrieve an existing team.
///
/// If you are using a personal access token, you need to have an access level greater or
/// equal to `member`.
///
/// `GET /v1/teams/{team_id}`, answering `200` Team details, `400` Invalid data, `401`
/// Invalid API key, `403` Forbidden Call, `404` Team not found or `503` API under
/// maintenance.
#[derive(Clone, Debug)]
pub struct RetrieveTeam {
    /// The id of the team.
    pub team_id: u32,
}

impl RetrieveTeam {
    pub fn new(team_id: u32) -> Self {
        Self { team_id }
    }
}

impl ApiCall for RetrieveTeam {
    type Output = Team;

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let url = config.endpoint(&format!("teams/{}", self.team_id))?;
        Ok(config.request(Method::GET, &url).body(Bytes::new())?)
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_json(response, StatusCode::OK)
    }
}

/// Body of a team update request
#[derive(Clone, Debug, Serialize)]
struct UpdateTeamBody<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
}

/// Update a team.
///
/// Update a team's name and/or its description.
///
/// If you are using a personal access token, you must have "can manage" permission on the
/// team or be a workspace manager.
///
/// The "All-incidents" team (`is_global=true`) cannot be updated.
///
/// `PATCH /v1/teams/{team_id}`, answering `200` The team was updated successfully, `400`
/// Invalid data, `401` Invalid API key or `503` API under maintenance.
#[derive(Clone, Debug)]
pub struct UpdateTeam {
    /// The id of the team.
    pub team_id: u32,
    pub name: Option<String>,
    /// Team description.
    pub description: Option<String>,
}

impl UpdateTeam {
    pub fn new(team_id: u32) -> Self {
        Self { team_id, name: None, description: None }
    }
}

impl ApiCall for UpdateTeam {
    type Output = Team;

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let url = config.endpoint(&format!("teams/{}", self.team_id))?;
        json_body(
            config.request(Method::PATCH, &url),
            &UpdateTeamBody {
                name: self.name.as_deref(),
                description: self.description.as_deref(),
            },
        )
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_json(response, StatusCode::OK)
    }
}

/// Delete a team.
///
/// Delete an existing team.
///
/// If you are using a personal access token, you must have "can manage" permission on the
/// team or be a workspace manager.
///
/// The "All-incidents" team (`is_global=true`) cannot be deleted.
///
/// `DELETE /v1/teams/{team_id}`, answering `204` The team was deleted successfully, `400`
/// Invalid data, `401` Invalid API key, `403` Forbidden Call, `404` Team not found or `503`
/// API under maintenance.
#[derive(Clone, Debug)]
pub struct DeleteTeam {
    /// The id of the team.
    pub team_id: u32,
}

impl DeleteTeam {
    pub fn new(team_id: u32) -> Self {
        Self { team_id }
    }
}

impl ApiCall for DeleteTeam {
    type Output = ();

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let url = config.endpoint(&format!("teams/{}", self.team_id))?;
        Ok(config.request(Method::DELETE, &url).body(Bytes::new())?)
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_no_content(response, StatusCode::NO_CONTENT)
    }
}
