use bytes::Bytes;
use http::{Method, Request, Response, StatusCode};
use serde::Serialize;

use crate::error::{ApiError, BuildError};
use crate::models::page::Page;
use crate::models::pagination::{Cursor, Pagination};
use crate::models::team::incident_permission::IncidentPermission;
use crate::models::team::invitation::TeamInvitation;
use crate::models::team::permission::TeamPermission;
use crate::util::json::{expect_json, expect_json_page, expect_no_content, json_body};
use crate::util::url::set_query;
use crate::{ApiCall, ApiConfig, Paginated};

/// Query parameters of a team invitation listing request
#[derive(Clone, Debug, Serialize)]
struct ListTeamInvitationsQueryParam<'a> {
    #[serde(flatten)]
    page: &'a Pagination,
    #[serde(skip_serializing_if = "Option::is_none")]
    invitation_id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_team_leader: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    team_permission: Option<TeamPermission>,
    #[serde(skip_serializing_if = "Option::is_none")]
    incident_permission: Option<IncidentPermission>,
}

/// List team invitations.
///
/// List all existing team invitations.
///
/// If you are using a personal access token, you must have "can manage" permission on the
/// team or be a workspace manager.
///
/// `GET /v1/teams/{team_id}/team_invitations`, answering `200` Team invitation list, `400`
/// Invalid data, `401` Invalid API key, `403` Permission denied, `404` Not found or `503`
/// API under maintenance.
#[derive(Clone, Debug)]
pub struct ListTeamInvitations {
    /// The id of the team.
    pub team_id: u32,
    pub page: Pagination,
    /// The id of an invitation to filter on.
    pub invitation_id: Option<u32>,
    /// Filter team invitations that will become team leaders.
    pub is_team_leader: Option<bool>,
    /// Filter team invitations with a specific team permission.
    ///
    /// `team_permission` is replaced by `is_team_leader`.
    pub team_permission: Option<TeamPermission>,
    /// Filter team invitations with a specific incident permission.
    pub incident_permission: Option<IncidentPermission>,
}

impl ListTeamInvitations {
    pub fn new(team_id: u32) -> Self {
        Self {
            team_id,
            page: Pagination::default(),
            invitation_id: None,
            is_team_leader: None,
            team_permission: None,
            incident_permission: None,
        }
    }
}

impl ApiCall for ListTeamInvitations {
    type Output = Page<TeamInvitation>;

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let mut url = config.endpoint(&format!("teams/{}/team_invitations", self.team_id))?;
        set_query(
            &mut url,
            &ListTeamInvitationsQueryParam {
                page: &self.page,
                invitation_id: self.invitation_id,
                is_team_leader: self.is_team_leader,
                team_permission: self.team_permission,
                incident_permission: self.incident_permission.clone(),
            },
        )?;
        Ok(config.request(Method::GET, &url).body(Bytes::new())?)
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_json_page(response, StatusCode::OK)
    }
}

impl Paginated for ListTeamInvitations {
    fn set_cursor(&mut self, cursor: Cursor) {
        self.page.cursor = Some(cursor);
    }
}

/// Body of a team invitation creation request
#[derive(Clone, Debug, Serialize)]
struct CreateTeamInvitationBody {
    invitation_id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_team_leader: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    team_permission: Option<TeamPermission>,
    #[serde(skip_serializing_if = "Option::is_none")]
    incident_permission: Option<IncidentPermission>,
}

/// Create a team invitation.
///
/// This endpoint allows you to create a team invitation from an existing team and
/// invitation.
///
/// If you are using a personal access token, you must have "can manage" permission on the
/// team or be a workspace manager.
///
/// `POST /v1/teams/{team_id}/team_invitations`, answering `201` Team invitation created,
/// `400` Invalid data, `401` Invalid API key, `403` Permission denied, `404` Not found,
/// `409` Data conflict or `503` API under maintenance.
#[derive(Clone, Debug)]
pub struct CreateTeamInvitation {
    /// The id of the team.
    pub team_id: u32,
    pub invitation_id: u32,
    pub is_team_leader: Option<bool>,
    /// `team_permission` is replaced by `is_team_leader`.
    pub team_permission: Option<TeamPermission>,
    pub incident_permission: Option<IncidentPermission>,
}

impl CreateTeamInvitation {
    pub fn new(team_id: u32, invitation_id: u32) -> Self {
        Self {
            team_id,
            invitation_id,
            is_team_leader: None,
            team_permission: None,
            incident_permission: None,
        }
    }
}

impl ApiCall for CreateTeamInvitation {
    type Output = TeamInvitation;

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let url = config.endpoint(&format!("teams/{}/team_invitations", self.team_id))?;
        json_body(
            config.request(Method::POST, &url),
            &CreateTeamInvitationBody {
                invitation_id: self.invitation_id,
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

/// Delete a team invitation.
///
/// Delete an existing team invitation.
///
/// If you are using a personal access token, you must have "can manage" permission on the
/// team or be a workspace manager.
///
/// `DELETE /v1/teams/{team_id}/team_invitations/{team_invitation_id}`, answering `204` Team
/// invitation was deleted successfully, `401` Invalid API key, `403` Permission denied,
/// `404` Not found or `503` API under maintenance.
#[derive(Clone, Debug)]
pub struct DeleteTeamInvitation {
    /// The id of the team.
    pub team_id: u32,
    /// The id of the team invitation.
    pub team_invitation_id: u32,
}

impl DeleteTeamInvitation {
    pub fn new(team_id: u32, team_invitation_id: u32) -> Self {
        Self { team_id, team_invitation_id }
    }
}

impl ApiCall for DeleteTeamInvitation {
    type Output = ();

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let url = config.endpoint(&format!(
            "teams/{}/team_invitations/{}",
            self.team_id, self.team_invitation_id
        ))?;
        Ok(config.request(Method::DELETE, &url).body(Bytes::new())?)
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_no_content(response, StatusCode::NO_CONTENT)
    }
}
