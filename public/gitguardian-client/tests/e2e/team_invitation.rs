//! Tests of /v1/teams/{team_id}/team_invitations

use crate::common::fixture::{
    create_team_invitation, delete_team_invitation, list_team_invitations,
};
use gitguardian_mock::MockServer;

#[cfg(feature = "ureq")]
mod ureq {
    use super::*;
    use crate::common::ureq::client;

    #[test]
    /// GIVEN a team id and filter parameters
    /// WHEN listing the team invitations
    /// THEN a page of team invitations is returned
    fn lists_team_invitations() {
        let server = MockServer::shared();

        let page = client(server)
            .send(&list_team_invitations())
            .expect("team invitation listing should succeed");

        assert!(!page.items.is_empty());
        assert!(page.items.iter().all(|invitation| invitation.invitation_id > 0));
        assert!(page.items.iter().all(|invitation| invitation.team_id > 0));
    }

    #[test]
    /// GIVEN a mock that always reports a next cursor
    /// WHEN taking two pages from the paginated call
    /// THEN both pages are fetched
    fn paginates_team_invitations() {
        let server = MockServer::shared();
        let client = client(server);

        let pages = client
            .paginate(list_team_invitations())
            .take(2)
            .collect::<Result<Vec<_>, _>>()
            .expect("team invitation pagination should succeed");

        assert_eq!(pages.len(), 2);
    }

    #[test]
    /// GIVEN a team id and an invitation id
    /// WHEN creating the team invitation
    /// THEN the created team invitation is returned
    fn creates_team_invitation() {
        let server = MockServer::shared();

        let invitation = client(server)
            .send(&create_team_invitation())
            .expect("team invitation creation should succeed");

        assert!(invitation.id > 0);
        assert!(invitation.invitation_id > 0);
    }

    #[test]
    /// GIVEN a team id and a team invitation id
    /// WHEN deleting it
    /// THEN the empty response is accepted
    fn deletes_team_invitation() {
        client(MockServer::shared())
            .send(&delete_team_invitation())
            .expect("deletion should succeed");
    }
}

#[cfg(feature = "reqwest")]
mod reqwest {
    use super::*;
    use crate::common::reqwest::client;
    use futures_util::{StreamExt, TryStreamExt};

    #[tokio::test]
    /// GIVEN a team id and filter parameters
    /// WHEN listing the team invitations
    /// THEN a page of team invitations is returned
    async fn lists_team_invitations() {
        let server = MockServer::shared();

        let page = client(server)
            .send(&list_team_invitations())
            .await
            .expect("team invitation listing should succeed");

        assert!(!page.items.is_empty());
        assert!(page.items.iter().all(|invitation| invitation.invitation_id > 0));
        assert!(page.items.iter().all(|invitation| invitation.team_id > 0));
    }

    #[tokio::test]
    /// GIVEN a mock that always reports a next cursor
    /// WHEN taking two pages from the paginated call
    /// THEN both pages are fetched
    async fn paginates_team_invitations() {
        let server = MockServer::shared();
        let client = client(server);

        let pages: Vec<_> = client
            .paginate(list_team_invitations())
            .take(2)
            .try_collect()
            .await
            .expect("team invitation pagination should succeed");

        assert_eq!(pages.len(), 2);
    }

    #[tokio::test]
    /// GIVEN a team id and an invitation id
    /// WHEN creating the team invitation
    /// THEN the created team invitation is returned
    async fn creates_team_invitation() {
        let server = MockServer::shared();

        let invitation = client(server)
            .send(&create_team_invitation())
            .await
            .expect("team invitation creation should succeed");

        assert!(invitation.id > 0);
        assert!(invitation.invitation_id > 0);
    }

    #[tokio::test]
    /// GIVEN a team id and a team invitation id
    /// WHEN deleting it
    /// THEN the empty response is accepted
    async fn deletes_team_invitation() {
        client(MockServer::shared())
            .send(&delete_team_invitation())
            .await
            .expect("deletion should succeed");
    }
}
