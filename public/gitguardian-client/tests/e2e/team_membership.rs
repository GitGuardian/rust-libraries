//! Tests of /v1/teams/{team_id}/team_memberships

use crate::common::fixture::{
    create_team_membership, delete_team_membership, list_team_memberships,
};
use gitguardian_mock::MockServer;

#[cfg(feature = "ureq")]
mod ureq {
    use super::*;
    use crate::common::ureq::client;

    #[test]
    /// GIVEN a team id and filter parameters
    /// WHEN listing the team memberships
    /// THEN a page of team memberships is returned
    fn lists_team_memberships() {
        let server = MockServer::shared();

        let page = client(server)
            .send(&list_team_memberships())
            .expect("team membership listing should succeed");

        assert!(!page.items.is_empty());
        assert!(page.items.iter().all(|membership| membership.member_id > 0));
        assert!(page.items.iter().all(|membership| membership.team_id > 0));
    }

    #[test]
    /// GIVEN a mock that always reports a next cursor
    /// WHEN taking two pages from the paginated call
    /// THEN both pages are fetched
    fn paginates_team_memberships() {
        let server = MockServer::shared();
        let client = client(server);

        let pages = client
            .paginate(list_team_memberships())
            .take(2)
            .collect::<Result<Vec<_>, _>>()
            .expect("team membership pagination should succeed");

        assert_eq!(pages.len(), 2);
    }

    #[test]
    /// GIVEN a team id and a member id
    /// WHEN adding the member to the team
    /// THEN the created team membership is returned
    fn creates_team_membership() {
        let server = MockServer::shared();

        let membership = client(server)
            .send(&create_team_membership())
            .expect("team membership creation should succeed");

        assert!(membership.id > 0);
        assert!(membership.member_id > 0);
    }

    #[test]
    /// GIVEN a team id and a team membership id
    /// WHEN deleting it
    /// THEN the empty response is accepted
    fn deletes_team_membership() {
        client(MockServer::shared())
            .send(&delete_team_membership())
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
    /// WHEN listing the team memberships
    /// THEN a page of team memberships is returned
    async fn lists_team_memberships() {
        let server = MockServer::shared();

        let page = client(server)
            .send(&list_team_memberships())
            .await
            .expect("team membership listing should succeed");

        assert!(!page.items.is_empty());
        assert!(page.items.iter().all(|membership| membership.member_id > 0));
        assert!(page.items.iter().all(|membership| membership.team_id > 0));
    }

    #[tokio::test]
    /// GIVEN a mock that always reports a next cursor
    /// WHEN taking two pages from the paginated call
    /// THEN both pages are fetched
    async fn paginates_team_memberships() {
        let server = MockServer::shared();
        let client = client(server);

        let pages: Vec<_> = client
            .paginate(list_team_memberships())
            .take(2)
            .try_collect()
            .await
            .expect("team membership pagination should succeed");

        assert_eq!(pages.len(), 2);
    }

    #[tokio::test]
    /// GIVEN a team id and a member id
    /// WHEN adding the member to the team
    /// THEN the created team membership is returned
    async fn creates_team_membership() {
        let server = MockServer::shared();

        let membership = client(server)
            .send(&create_team_membership())
            .await
            .expect("team membership creation should succeed");

        assert!(membership.id > 0);
        assert!(membership.member_id > 0);
    }

    #[tokio::test]
    /// GIVEN a team id and a team membership id
    /// WHEN deleting it
    /// THEN the empty response is accepted
    async fn deletes_team_membership() {
        client(MockServer::shared())
            .send(&delete_team_membership())
            .await
            .expect("deletion should succeed");
    }
}
