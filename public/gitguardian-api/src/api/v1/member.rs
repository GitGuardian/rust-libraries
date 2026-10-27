use bytes::Bytes;
use http::{Method, Request, Response, StatusCode};
use serde::Serialize;

use crate::error::{ApiError, BuildError};
use crate::models::member::Member;
use crate::models::member::access_level::AccessLevel;
use crate::models::member::ordering::MemberOrdering;
use crate::models::page::Page;
use crate::models::pagination::{Cursor, Pagination};
use crate::util::json::{expect_json, expect_json_page, expect_no_content, json_body};
use crate::util::url::set_query;
use crate::{ApiCall, ApiConfig, Paginated};

/// Query parameters of a member listing request
#[derive(Clone, Debug, Serialize)]
struct ListMembersQueryParam<'a> {
    #[serde(flatten)]
    page: &'a Pagination,
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<AccessLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    access_level: Option<AccessLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    search: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ordering: Option<MemberOrdering>,
}

/// List members.
///
/// List members of the workspace.
///
/// `GET /v1/members`, answering `200` List members, `400` Invalid data, `401` Invalid API
/// key or `503` API under maintenance.
#[derive(Clone, Debug, Default)]
pub struct ListMembers {
    pub page: Pagination,
    /// Filter members based on their access level. Use `access_level` instead.
    pub role: Option<AccessLevel>,
    /// Filter members based on their access level.
    pub access_level: Option<AccessLevel>,
    /// Filter members based on their active status.
    pub active: Option<bool>,
    /// Search members based on their name or email.
    pub search: Option<String>,
    /// Sort the results by their field value. The default sort is ASC, DESC if the field
    /// is preceded by a `-`.
    pub ordering: Option<MemberOrdering>,
}

impl ApiCall for ListMembers {
    type Output = Page<Member>;

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let mut url = config.endpoint("members")?;
        set_query(
            &mut url,
            &ListMembersQueryParam {
                page: &self.page,
                role: self.role,
                access_level: self.access_level,
                active: self.active,
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

impl Paginated for ListMembers {
    fn set_cursor(&mut self, cursor: Cursor) {
        self.page.cursor = Some(cursor);
    }
}

/// Retrieve a member.
///
/// Retrieve an existing workspace member.
///
/// If you are using a personal access token, you need to have an access level greater or
/// equal to `member`.
///
/// `GET /v1/members/{member_id}`, answering `200` Workspace member details, `400` Invalid
/// data, `401` Invalid API key, `403` Forbidden Call, `404` Member not found or `503` API
/// under maintenance.
#[derive(Clone, Debug)]
pub struct RetrieveMember {
    /// The id of the workspace member.
    pub member_id: u32,
}

impl RetrieveMember {
    pub fn new(member_id: u32) -> Self {
        Self { member_id }
    }
}

impl ApiCall for RetrieveMember {
    type Output = Member;

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let url = config.endpoint(&format!("members/{}", self.member_id))?;
        Ok(config.request(Method::GET, &url).body(Bytes::new())?)
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_json(response, StatusCode::OK)
    }
}

/// Query parameters of a member update request
#[derive(Clone, Debug, Serialize)]
struct UpdateMemberQueryParam {
    #[serde(skip_serializing_if = "Option::is_none")]
    send_email: Option<bool>,
}

/// Body of a member update request
#[derive(Clone, Debug, Serialize)]
struct UpdateMemberBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    access_level: Option<AccessLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    active: Option<bool>,
}

/// Update a member.
///
/// Update an existing workspace member.
///
/// If you are using a personal access token, you need to have an access level greater or
/// equal to `manager`.
///
/// `PATCH /v1/members/{member_id}`, answering `200` Workspace member details, `400` Invalid
/// data, `401` Invalid API key, `403` Forbidden Call, `404` Member not found or `503` API
/// under maintenance.
#[derive(Clone, Debug)]
pub struct UpdateMember {
    /// The id of the workspace member.
    pub member_id: u32,
    pub access_level: Option<AccessLevel>,
    pub active: Option<bool>,
    /// Whether to notify the member about the update.
    pub send_email: Option<bool>,
}

impl UpdateMember {
    pub fn new(member_id: u32) -> Self {
        Self { member_id, access_level: None, active: None, send_email: None }
    }
}

impl ApiCall for UpdateMember {
    type Output = Member;

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let mut url = config.endpoint(&format!("members/{}", self.member_id))?;
        set_query(&mut url, &UpdateMemberQueryParam { send_email: self.send_email })?;
        json_body(
            config.request(Method::PATCH, &url),
            &UpdateMemberBody { access_level: self.access_level, active: self.active },
        )
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_json(response, StatusCode::OK)
    }
}

/// Query parameters of a member deletion request
#[derive(Clone, Debug, Serialize)]
struct DeleteMemberQueryParam {
    #[serde(skip_serializing_if = "Option::is_none")]
    send_email: Option<bool>,
}

/// Delete a member.
///
/// Delete an existing workspace member.
///
/// If you are using a personal access token, you need to have an access level greater or
/// equal to `manager`.
///
/// `DELETE /v1/members/{member_id}`, answering `204` The member was deleted successfully,
/// `400` Invalid data, `401` Invalid API key, `403` Forbidden Call, `404` Member not found
/// or `503` API under maintenance.
#[derive(Clone, Debug)]
pub struct DeleteMember {
    /// The id of the workspace member.
    pub member_id: u32,
    /// Whether to notify the member about the removal.
    pub send_email: Option<bool>,
}

impl DeleteMember {
    pub fn new(member_id: u32) -> Self {
        Self { member_id, send_email: None }
    }
}

impl ApiCall for DeleteMember {
    type Output = ();

    fn build(&self, config: &ApiConfig) -> Result<Request<Bytes>, BuildError> {
        let mut url = config.endpoint(&format!("members/{}", self.member_id))?;
        set_query(&mut url, &DeleteMemberQueryParam { send_email: self.send_email })?;
        Ok(config.request(Method::DELETE, &url).body(Bytes::new())?)
    }

    fn parse(&self, response: Response<Bytes>) -> Result<Self::Output, ApiError> {
        expect_no_content(response, StatusCode::NO_CONTENT)
    }
}
