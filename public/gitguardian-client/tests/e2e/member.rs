//! Tests of /v1/members

use crate::common::fixture::{delete_member, list_members, retrieve_member, update_member};
use gitguardian_mock::MockServer;

#[cfg(feature = "ureq")]
mod ureq {
    use super::*;
    use crate::common::ureq::client;

    #[test]
    /// GIVEN pagination and filter parameters
    /// WHEN listing members
    /// THEN a page of members is returned
    fn lists_members() {
        let server = MockServer::shared();

        let page = client(server).send(&list_members()).expect("member listing should succeed");

        assert!(!page.items.is_empty());
        assert!(page.items.iter().all(|member| !member.email.is_empty()));
    }

    #[test]
    /// GIVEN a mock that always reports a next cursor
    /// WHEN taking two pages from the paginated call
    /// THEN both pages are fetched
    fn paginates_members() {
        let server = MockServer::shared();
        let client = client(server);

        let pages = client
            .paginate(list_members())
            .take(2)
            .collect::<Result<Vec<_>, _>>()
            .expect("member pagination should succeed");

        assert_eq!(pages.len(), 2);
    }

    #[test]
    /// GIVEN a member id
    /// WHEN retrieving that member
    /// THEN the member is returned
    fn retrieves_member() {
        let server = MockServer::shared();

        let member =
            client(server).send(&retrieve_member()).expect("member retrieval should succeed");

        assert!(member.id > 0);
        assert!(!member.email.is_empty());
        assert_eq!(member.role, member.access_level);
    }

    #[test]
    /// GIVEN a member id and fields to change
    /// WHEN updating that member
    /// THEN the updated member is returned
    fn updates_member() {
        let member = client(MockServer::shared())
            .send(&update_member())
            .expect("member update should succeed");

        assert!(member.id > 0);
        assert!(!member.email.is_empty());
    }

    #[test]
    /// GIVEN a member id
    /// WHEN deleting it
    /// THEN the empty response is accepted
    fn deletes_member() {
        client(MockServer::shared()).send(&delete_member()).expect("deletion should succeed");
    }
}

#[cfg(feature = "reqwest")]
mod reqwest {
    use super::*;
    use crate::common::reqwest::client;
    use futures_util::{StreamExt, TryStreamExt};

    #[tokio::test]
    /// GIVEN pagination and filter parameters
    /// WHEN listing members
    /// THEN a page of members is returned
    async fn lists_members() {
        let server = MockServer::shared();

        let page =
            client(server).send(&list_members()).await.expect("member listing should succeed");

        assert!(!page.items.is_empty());
        assert!(page.items.iter().all(|member| !member.email.is_empty()));
    }

    #[tokio::test]
    /// GIVEN a mock that always reports a next cursor
    /// WHEN taking two pages from the paginated call
    /// THEN both pages are fetched
    async fn paginates_members() {
        let server = MockServer::shared();
        let client = client(server);

        let pages: Vec<_> = client
            .paginate(list_members())
            .take(2)
            .try_collect()
            .await
            .expect("member pagination should succeed");

        assert_eq!(pages.len(), 2);
    }

    #[tokio::test]
    /// GIVEN a member id
    /// WHEN retrieving that member
    /// THEN the member is returned
    async fn retrieves_member() {
        let server = MockServer::shared();

        let member =
            client(server).send(&retrieve_member()).await.expect("member retrieval should succeed");

        assert!(member.id > 0);
        assert!(!member.email.is_empty());
        assert_eq!(member.role, member.access_level);
    }

    #[tokio::test]
    /// GIVEN a member id and fields to change
    /// WHEN updating that member
    /// THEN the updated member is returned
    async fn updates_member() {
        let member = client(MockServer::shared())
            .send(&update_member())
            .await
            .expect("member update should succeed");

        assert!(member.id > 0);
        assert!(!member.email.is_empty());
    }

    #[tokio::test]
    /// GIVEN a member id
    /// WHEN deleting it
    /// THEN the empty response is accepted
    async fn deletes_member() {
        client(MockServer::shared()).send(&delete_member()).await.expect("deletion should succeed");
    }
}
