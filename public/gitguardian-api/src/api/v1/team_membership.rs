use bytes::Bytes;
use http::{Method, Request, Response, StatusCode};
use serde::Serialize;

use crate::error::{ApiError, BuildError};
use crate::models::page::Page;
use crate::models::pagination::{Cursor, Pagination};
use crate::models::team::incident_permission::IncidentPermission;
use crate::models::team::membership::TeamMembership;
use crate::models::team::permission::TeamPermission;
use crate::util::json::{expect_json, expect_json_page, expect_no_content, json_body};
use crate::util::url::set_query;
use crate::{ApiCall, ApiConfig, Paginated};

/// Query parameters of a team membership listing request
#[derive(Clone, Debug, Serialize)]
struct ListTeamMembershipsQueryParam<'a> {
    #[serde(flatten)]
    page: &'a Pagination,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_team_leader: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    team_permission: Option<TeamPermission>,
    #[serde(skip_serializing_if = "Option::is_none")]
    incident_permission: Option<IncidentPermission>,
    #[serde(skip_serializing_if = "Option::is_none")]
    member_id: Option<u32>,
}

/// List team memberships.
///
/// List all the memberships of a team.
///
/// If you are using a personal access token, you need to be a workspace manager or be part
/// of the team.
///
/// `GET /v1/teams/{team_id}/team_memberships`, answering `200` Team membership list, `400`
/// Invalid data, `401` Invalid API key, `403` Permission denied, `404` Not found or `503`
/// API under maintenance.
#[derive(Clone, Debug)]
pub struct ListTeamMemberships {
    /// The id of the team.
    pub team_id: u32,
    pub page: Pagination,
    /// Filter team memberships that are team leaders.
    pub is_team_leader: Option<bool>,
    /// Filter team memberships with a specific team permission.
    ///
    /// `team_permission` is replaced by `is_team_leader`.
    pub team_permission: Option<TeamPermission>,
    /// Filter team memberships with a specific incident permission.
    pub incident_permission: Option<IncidentPermission>,
    /// Filter team memberships on a specific member.
    pub member_id: Option<u32>,
}

impl ListTeamMemberships {
    pub fn new(team_id: u32) -> Self {
        Self {
            team_id,
            page: Pagination::default(),
            is_team_leader: None,
            team_permission: None,
            incident_permission: None,
            member_id: None,
        }
    }
}

impl ApiCall for ListTeamMemberships {
    type Output = Page<TeamMembership>;

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let mut url = config.endpoint(&format!("teams/{}/team_memberships", self.team_id))?;
        set_query(
            &mut url,
            &ListTeamMembershipsQueryParam {
                page: &self.page,
                is_team_leader: self.is_team_leader,
                team_permission: self.team_permission,
                incident_permission: self.incident_permission.clone(),
                member_id: self.member_id,
            },
        )?;
        Ok(config.request(Method::GET, &url).body(Bytes::new())?)
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_json_page(response, StatusCode::OK)
    }
}

impl Paginated for ListTeamMemberships {
    fn set_cursor(&mut self, cursor: Cursor) {
        self.page.cursor = Some(cursor);
    }
}

/// Query parameters of a team membership creation request
#[derive(Clone, Debug, Serialize)]
struct CreateTeamMembershipQueryParam {
    #[serde(skip_serializing_if = "Option::is_none")]
    send_email: Option<bool>,
}

/// Body of a team membership creation request
#[derive(Clone, Debug, Serialize)]
struct CreateTeamMembershipBody {
    member_id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_team_leader: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    team_permission: Option<TeamPermission>,
    #[serde(skip_serializing_if = "Option::is_none")]
    incident_permission: Option<IncidentPermission>,
}

/// Add a member to a team.
///
/// Add a member to a team.
///
/// If you are using a personal access token, you must have "can manage" permission on the
/// team or be a workspace manager.
///
/// `POST /v1/teams/{team_id}/team_memberships`, answering `201` Team membership created,
/// `400` Invalid data, `401` Invalid API key, `403` Permission denied, `404` Not found,
/// `409` Data conflict or `503` API under maintenance.
#[derive(Clone, Debug)]
pub struct CreateTeamMembership {
    /// The id of the team.
    pub team_id: u32,
    /// Id of a workspace member.
    pub member_id: u32,
    pub is_team_leader: Option<bool>,
    /// `team_permission` is replaced by `is_team_leader`.
    pub team_permission: Option<TeamPermission>,
    pub incident_permission: Option<IncidentPermission>,
    /// Whether to notify the member about the team membership.
    pub send_email: Option<bool>,
}

impl CreateTeamMembership {
    pub fn new(team_id: u32, member_id: u32) -> Self {
        Self {
            team_id,
            member_id,
            is_team_leader: None,
            team_permission: None,
            incident_permission: None,
            send_email: None,
        }
    }
}

impl ApiCall for CreateTeamMembership {
    type Output = TeamMembership;

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let mut url = config.endpoint(&format!("teams/{}/team_memberships", self.team_id))?;
        set_query(&mut url, &CreateTeamMembershipQueryParam { send_email: self.send_email })?;
        json_body(
            config.request(Method::POST, &url),
            &CreateTeamMembershipBody {
                member_id: self.member_id,
                is_team_leader: self.is_team_leader,
                team_permission: self.team_permission,
                incident_permission: self.incident_permission.clone(),
            },
        )
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_json(response, StatusCode::CREATED)
    }
}

/// Query parameters of a team membership deletion request
#[derive(Clone, Debug, Serialize)]
struct DeleteTeamMembershipQueryParam {
    #[serde(skip_serializing_if = "Option::is_none")]
    send_email: Option<bool>,
}

/// Remove a member from a team.
///
/// Remove a member from a team.
///
/// If you are using a personal access token, you must have "can manage" permission on the
/// team or be a workspace manager, or be the member being removed.
///
/// `DELETE /v1/teams/{team_id}/team_memberships/{team_membership_id}`, answering `204` Team
/// membership was deleted successfully, `401` Invalid API key, `403` Permission denied,
/// `404` Not found or `503` API under maintenance.
#[derive(Clone, Debug)]
pub struct DeleteTeamMembership {
    /// The id of the team.
    pub team_id: u32,
    /// The id of the team membership.
    pub team_membership_id: u32,
    /// Whether to notify the member about the removal from the team.
    pub send_email: Option<bool>,
}

impl DeleteTeamMembership {
    pub fn new(team_id: u32, team_membership_id: u32) -> Self {
        Self { team_id, team_membership_id, send_email: None }
    }
}

impl ApiCall for DeleteTeamMembership {
    type Output = ();

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let mut url = config.endpoint(&format!(
            "teams/{}/team_memberships/{}",
            self.team_id, self.team_membership_id
        ))?;
        set_query(&mut url, &DeleteTeamMembershipQueryParam { send_email: self.send_email })?;
        Ok(config.request(Method::DELETE, &url).body(Bytes::new())?)
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_no_content(response, StatusCode::NO_CONTENT)
    }
}
