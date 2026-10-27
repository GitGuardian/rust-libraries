use bytes::Bytes;
use http::{Method, Request, Response, StatusCode};
use serde::Serialize;

use crate::error::{ApiError, BuildError};
use crate::models::invitation::Invitation;
use crate::models::invitation::access_level::InvitationAccessLevel;
use crate::models::invitation::ordering::InvitationOrdering;
use crate::models::page::Page;
use crate::models::pagination::{Cursor, Pagination};
use crate::util::json::{expect_json, expect_json_page, expect_no_content, json_body};
use crate::util::url::set_query;
use crate::{ApiCall, ApiConfig, Paginated};

/// Query parameters for an invitation creation request
#[derive(Clone, Debug, Serialize)]
struct CreateInvitationQueryParam {
    #[serde(skip_serializing_if = "Option::is_none")]
    send_email: Option<bool>,
}

/// Body of an invitation creation request
#[derive(Clone, Debug, Serialize)]
struct CreateInvitationBody<'a> {
    email: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    access_level: Option<InvitationAccessLevel>,
}

/// Create an invitation.
///
/// This endpoint allows you to send an invitation to a user.
///
/// If you are using a personal access token, you need to have an access level superior or
/// equal to `member`.
///
/// `POST /v1/invitations`, answering `201` Invitation sent, `400` Invalid data, `401`
/// Invalid API key, `403` Forbidden Call, `409` Data conflict or `503` API under
/// maintenance.
#[derive(Clone, Debug)]
pub struct CreateInvitation {
    pub email: String,
    pub access_level: Option<InvitationAccessLevel>,
    /// Whether to send an email to the invitee with a link to accept the invitation.
    pub send_email: Option<bool>,
}

impl CreateInvitation {
    pub fn new(email: impl Into<String>) -> Self {
        Self { email: email.into(), access_level: None, send_email: None }
    }
}

impl ApiCall for CreateInvitation {
    type Output = Invitation;

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let mut url = config.endpoint("invitations")?;
        set_query(&mut url, &CreateInvitationQueryParam { send_email: self.send_email })?;
        json_body(
            config.request(Method::POST, &url),
            &CreateInvitationBody { email: &self.email, access_level: self.access_level },
        )
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_json(response, StatusCode::CREATED)
    }
}

/// Query parameters of an invitation listing request
#[derive(Clone, Debug, Serialize)]
struct ListInvitationsQueryParam<'a> {
    #[serde(flatten)]
    page: &'a Pagination,
    #[serde(skip_serializing_if = "Option::is_none")]
    search: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ordering: Option<InvitationOrdering>,
}

/// List invitations.
///
/// This endpoint allows you to list all pending invitations.
///
/// The response contains the list of invitations and a pagination cursor to retrieve the
/// next page.
///
/// The invitations are sorted by id.
///
/// If you are using a personal access token, you need to have an access level superior or
/// equal to `member`.
///
/// `GET /v1/invitations`, answering `200` List invitations, `400` Invalid data, `401`
/// Invalid API key, `403` Forbidden Call or `503` API under maintenance.
#[derive(Clone, Debug, Default)]
pub struct ListInvitations {
    pub page: Pagination,
    /// Search invitations based on the email field.
    pub search: Option<String>,
    /// Sort the results by their field value. The default sort is ASC, DESC if the field
    /// is preceded by a `-`.
    pub ordering: Option<InvitationOrdering>,
}

impl ApiCall for ListInvitations {
    type Output = Page<Invitation>;

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let mut url = config.endpoint("invitations")?;
        set_query(
            &mut url,
            &ListInvitationsQueryParam {
                page: &self.page,
                search: self.search.as_deref(),
                ordering: self.ordering,
            },
        )?;
        Ok(config.request(Method::GET, &url).body(Bytes::new())?)
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_json_page(response, StatusCode::OK)
    }
}

impl Paginated for ListInvitations {
    fn set_cursor(&mut self, cursor: Cursor) {
        self.page.cursor = Some(cursor);
    }
}

/// Delete an invitation.
///
/// Delete an existing invitation.
///
/// If you are using a personal access token, you need to have an access level superior or
/// equal to `manager`.
///
/// `DELETE /v1/invitations/{invitation_id}`, answering `204` The invitation was deleted
/// successfully, `400` Invalid data, `401` Invalid API key, `403` Forbidden Call, `404`
/// Invitation not found or `503` API under maintenance.
#[derive(Clone, Debug)]
pub struct DeleteInvitation {
    /// The id of the invitation.
    pub invitation_id: u32,
}

impl DeleteInvitation {
    pub fn new(invitation_id: u32) -> Self {
        Self { invitation_id }
    }
}

impl ApiCall for DeleteInvitation {
    type Output = ();

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let url = config.endpoint(&format!("invitations/{}", self.invitation_id))?;
        Ok(config.request(Method::DELETE, &url).body(Bytes::new())?)
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_no_content(response, StatusCode::NO_CONTENT)
    }
}
