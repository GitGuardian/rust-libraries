use gitguardian_api::api::v1::api_token::{
    CreateApiToken, ListApiTokens, RetrieveApiToken, RetrieveCurrentApiToken, RevokeApiToken,
    RevokeCurrentApiToken,
};
use gitguardian_api::api::v1::custom_host::ListDetectorCustomHosts;
use gitguardian_api::api::v1::endpoint_deployment::{
    ConfirmEndpointDeployment, CreateEndpointDeployment, ListEndpointDeployments, MachineInfo,
};
use gitguardian_api::api::v1::health::CheckHealth;
use gitguardian_api::api::v1::honeytoken::CreateHoneytoken;
use gitguardian_api::api::v1::honeytoken_prefix::CheckHoneytokenPrefixes;
use gitguardian_api::api::v1::honeytoken_with_context::CreateHoneytokenWithContext;
use gitguardian_api::api::v1::invitation::{CreateInvitation, DeleteInvitation, ListInvitations};
use gitguardian_api::api::v1::jwt::CreateJwt;
use gitguardian_api::api::v1::member::{DeleteMember, ListMembers, RetrieveMember, UpdateMember};
use gitguardian_api::api::v1::metadata::RetrieveMetadata;
use gitguardian_api::api::v1::multiscan::MultiScan;
use gitguardian_api::api::v1::oauth::CreateOAuthToken;
use gitguardian_api::api::v1::quota::RetrieveQuotas;
use gitguardian_api::api::v1::scan::Scan;
use gitguardian_api::api::v1::scan_create_incidents::ScanCreateIncidents;
use gitguardian_api::api::v1::secret_incident::RetrieveSecretIncident;
use gitguardian_api::api::v1::source::ListSources;
use gitguardian_api::api::v1::team::{CreateTeam, DeleteTeam, ListTeams, RetrieveTeam, UpdateTeam};
use gitguardian_api::api::v1::team_invitation::{
    CreateTeamInvitation, DeleteTeamInvitation, ListTeamInvitations,
};
use gitguardian_api::api::v1::team_membership::{
    CreateTeamMembership, DeleteTeamMembership, ListTeamMemberships,
};
use gitguardian_api::api::v1::team_source::{ListTeamSources, UpdateTeamSources};
use gitguardian_api::models::api_token::ordering::ApiTokenOrdering;
use gitguardian_api::models::api_token::scope::ApiTokenScope;
use gitguardian_api::models::api_token::status::ApiTokenStatus;
use gitguardian_api::models::api_token::r#type::ApiTokenType;
use gitguardian_api::models::document::Document;
use gitguardian_api::models::document_location::DocumentLocation;
use gitguardian_api::models::endpoint_deployment::status::DeploymentStatus;
use gitguardian_api::models::honeytoken::r#type::HoneytokenType;
use gitguardian_api::models::invitation::ordering::InvitationOrdering;
use gitguardian_api::models::member::access_level::AccessLevel;
use gitguardian_api::models::member::ordering::MemberOrdering;
use gitguardian_api::models::pagination::Pagination;
use gitguardian_api::models::source::health::SourceHealth;
use gitguardian_api::models::source::ordering::SourceOrdering;
use gitguardian_api::models::team::incident_permission::IncidentPermission;
use gitguardian_api::models::team::permission::TeamPermission;
use gitguardian_api::uuid::Uuid;

pub const SOURCE_UUID: Uuid = Uuid::from_u128(0x550e8400_e29b_41d4_a716_446655440000);

pub const TEAM_ID: u32 = 3252;

pub const MEMBER_ID: u32 = 3252;

pub const INVITATION_ID: u32 = 4851;

pub const SOURCE_ID: u32 = 6531;

pub const TOKEN_ID: Uuid = Uuid::from_u128(0x5ddaad0c_5a0c_4674_beb5_1cd198d13360);

pub const TEAM_INVITATION_ID: u32 = 3252;

pub const TEAM_MEMBERSHIP_ID: u32 = 1234;

pub const INCIDENT_ID: u32 = 3759;

pub fn document() -> Document {
    Document::new("aws_key = AKIA123").with_filename("intro.py")
}

pub fn documents() -> Vec<Document> {
    vec![document(), Document::new("__version__ = \"1.0.0\"").with_filename("tasks.py")]
}

pub fn scan() -> Scan {
    document().into()
}

pub fn multiscan() -> MultiScan {
    documents().into_iter().collect()
}

pub fn scan_create_incidents() -> ScanCreateIncidents {
    ScanCreateIncidents::new(
        SOURCE_UUID,
        vec![
            document()
                .with_location(DocumentLocation::new("https://wiki.example.com/my-config-page")),
        ],
    )
}

pub fn create_honeytoken_with_context() -> CreateHoneytokenWithContext {
    let mut call = CreateHoneytokenWithContext::new("honeytoken A", HoneytokenType::Aws);
    call.language = Some("python".to_owned());
    call.project_extensions = vec![".py".to_owned(), ".c".to_owned()];
    call
}

pub fn create_jwt() -> CreateJwt {
    CreateJwt::new("https://api.hasmysecretleaked.com")
}

pub fn create_honeytoken() -> CreateHoneytoken {
    CreateHoneytoken::new("honeytoken A", HoneytokenType::Aws)
}

pub fn create_team() -> CreateTeam {
    CreateTeam::new("feature team A")
}

pub fn list_teams() -> ListTeams {
    ListTeams {
        page: Pagination { per_page: Some(1), ..Default::default() },
        is_global: Some(false),
        ..Default::default()
    }
}

pub fn retrieve_team() -> RetrieveTeam {
    RetrieveTeam::new(TEAM_ID)
}

pub fn list_invitations() -> ListInvitations {
    ListInvitations {
        page: Pagination { per_page: Some(1), ..Default::default() },
        ordering: Some(InvitationOrdering::DateDesc),
        ..Default::default()
    }
}

pub fn list_members() -> ListMembers {
    ListMembers {
        page: Pagination { per_page: Some(1), ..Default::default() },
        access_level: Some(AccessLevel::Manager),
        active: Some(true),
        ordering: Some(MemberOrdering::CreatedAtDesc),
        ..Default::default()
    }
}

pub fn retrieve_member() -> RetrieveMember {
    RetrieveMember::new(MEMBER_ID)
}

pub fn list_team_invitations() -> ListTeamInvitations {
    let mut call = ListTeamInvitations::new(TEAM_ID);
    call.page.per_page = Some(1);
    call.incident_permission = Some(IncidentPermission::FullAccess);
    call
}

pub fn list_team_memberships() -> ListTeamMemberships {
    let mut call = ListTeamMemberships::new(TEAM_ID);
    call.page.per_page = Some(1);
    call.team_permission = Some(TeamPermission::CanManage);
    call
}

pub fn create_invitation() -> CreateInvitation {
    CreateInvitation::new("someone@example.com")
}

pub fn create_team_invitation() -> CreateTeamInvitation {
    let mut call = CreateTeamInvitation::new(TEAM_ID, INVITATION_ID);
    call.is_team_leader = Some(false);
    call.incident_permission = Some(IncidentPermission::CanEdit);
    call
}

pub fn create_team_membership() -> CreateTeamMembership {
    let mut call = CreateTeamMembership::new(TEAM_ID, MEMBER_ID);
    call.is_team_leader = Some(false);
    call.incident_permission = Some(IncidentPermission::CanEdit);
    call.send_email = Some(false);
    call
}

pub fn update_team_sources() -> UpdateTeamSources {
    let mut call = UpdateTeamSources::new(TEAM_ID);
    call.sources_to_add = vec![SOURCE_ID];
    call
}

pub fn check_health() -> CheckHealth {
    CheckHealth
}

pub fn retrieve_quotas() -> RetrieveQuotas {
    RetrieveQuotas
}

pub fn retrieve_current_api_token() -> RetrieveCurrentApiToken {
    RetrieveCurrentApiToken
}

pub fn retrieve_api_token() -> RetrieveApiToken {
    RetrieveApiToken::new(TOKEN_ID)
}

pub fn list_sources() -> ListSources {
    ListSources {
        page: Pagination { per_page: Some(1), ..Default::default() },
        health: Some(SourceHealth::AtRisk),
        monitored: Some(true),
        ordering: Some(SourceOrdering::LastScanDateDesc),
        ..Default::default()
    }
}

pub fn list_team_sources() -> ListTeamSources {
    let mut call = ListTeamSources::new(TEAM_ID);
    call.page.per_page = Some(1);
    call.health = Some(SourceHealth::Safe);
    call
}

pub fn update_member() -> UpdateMember {
    let mut call = UpdateMember::new(MEMBER_ID);
    call.access_level = Some(AccessLevel::Member);
    call.active = Some(true);
    call.send_email = Some(false);
    call
}

pub fn update_team() -> UpdateTeam {
    let mut call = UpdateTeam::new(TEAM_ID);
    call.name = Some("feature team B".to_owned());
    call.description = Some("Description of my team".to_owned());
    call
}

pub fn delete_member() -> DeleteMember {
    let mut call = DeleteMember::new(MEMBER_ID);
    call.send_email = Some(false);
    call
}

pub fn delete_team() -> DeleteTeam {
    DeleteTeam::new(TEAM_ID)
}

pub fn delete_invitation() -> DeleteInvitation {
    DeleteInvitation::new(INVITATION_ID)
}

pub fn delete_team_invitation() -> DeleteTeamInvitation {
    DeleteTeamInvitation::new(TEAM_ID, TEAM_INVITATION_ID)
}

pub fn delete_team_membership() -> DeleteTeamMembership {
    let mut call = DeleteTeamMembership::new(TEAM_ID, TEAM_MEMBERSHIP_ID);
    call.send_email = Some(false);
    call
}

pub fn retrieve_secret_incident() -> RetrieveSecretIncident {
    let mut call = RetrieveSecretIncident::new(INCIDENT_ID);
    call.with_occurrences = Some(1);
    call
}

pub fn retrieve_metadata() -> RetrieveMetadata {
    RetrieveMetadata
}

pub fn list_detector_custom_hosts() -> ListDetectorCustomHosts {
    ListDetectorCustomHosts
}

pub const MACHINE_ID: &str = "7e3a9d7f-8a5e-4e23-9c2f-eb1d6f64fa55";

pub const USERNAME: &str = "alice";

pub const DEPLOYMENT_ID: &str = "800172b9-5002-43c6-bf5b-7112afc59721";

pub fn create_oauth_token() -> CreateOAuthToken {
    let mut call = CreateOAuthToken::new(
        "4/0Adeu5BWqv9oS",
        "https://app.example.com/callback",
        "gg_client_AbCdEf123456",
        "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk",
    );
    call.name = Some("My MCP client".to_owned());
    call.lifetime = Some(30);
    call
}

pub fn check_honeytoken_prefixes() -> CheckHoneytokenPrefixes {
    ["abcde".to_owned(), "12345".to_owned()].into_iter().collect()
}

pub fn machine_info() -> MachineInfo {
    MachineInfo::new(MACHINE_ID, USERNAME, "alice-laptop")
}

pub fn create_endpoint_deployment() -> CreateEndpointDeployment {
    let mut call = CreateEndpointDeployment::new(machine_info());
    call.description = Some("Deployed by ggshield on the CI runner".to_owned());
    call
}

pub fn list_endpoint_deployments() -> ListEndpointDeployments {
    ListEndpointDeployments::new(MACHINE_ID, USERNAME)
}

pub fn confirm_endpoint_deployment() -> ConfirmEndpointDeployment {
    ConfirmEndpointDeployment::new(DEPLOYMENT_ID, DeploymentStatus::Planted)
}

pub fn list_api_tokens() -> ListApiTokens {
    ListApiTokens {
        page: Pagination { per_page: Some(1), ..Default::default() },
        status: Some(ApiTokenStatus::Active),
        scopes: Some(ApiTokenScope::Scan),
        ordering: Some(ApiTokenOrdering::CreatedAtDesc),
        ..Default::default()
    }
}

pub fn create_api_token() -> CreateApiToken {
    CreateApiToken::new(
        "myTokenName",
        ApiTokenType::PersonalAccessToken,
        vec![ApiTokenScope::Scan, ApiTokenScope::IncidentsRead],
        30,
    )
}

pub fn revoke_current_api_token() -> RevokeCurrentApiToken {
    RevokeCurrentApiToken
}

pub fn revoke_api_token() -> RevokeApiToken {
    RevokeApiToken::new(TOKEN_ID)
}
