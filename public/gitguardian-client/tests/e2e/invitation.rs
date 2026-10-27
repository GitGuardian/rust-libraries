//! Tests of /v1/invitations

use crate::common::fixture::{create_invitation, delete_invitation, list_invitations};
use gitguardian_api::models::invitation::access_level::InvitationAccessLevel;
use gitguardian_mock::MockServer;

#[cfg(feature = "ureq")]
mod ureq {
    use super::*;
    use crate::common::ureq::client;

    #[test]
    /// GIVEN an email address
    /// WHEN creating an invitation for it
    /// THEN the created invitation is returned
    fn creates_invitation() {
        let server = MockServer::shared();

        let invitation =
            client(server).send(&create_invitation()).expect("invitation creation should succeed");

        assert!(!invitation.email.is_empty());
        assert_eq!(invitation.access_level, InvitationAccessLevel::Manager);
    }

    #[test]
    /// GIVEN pagination and ordering parameters
    /// WHEN listing invitations
    /// THEN a page of invitations is returned
    fn lists_invitations() {
        let server = MockServer::shared();

        let page =
            client(server).send(&list_invitations()).expect("invitation listing should succeed");

        assert!(!page.items.is_empty());
        assert!(page.items.iter().all(|invitation| !invitation.email.is_empty()));
    }

    #[test]
    /// GIVEN an invitation id
    /// WHEN deleting it
    /// THEN the empty response is accepted
    fn deletes_invitation() {
        client(MockServer::shared()).send(&delete_invitation()).expect("deletion should succeed");
    }
}

#[cfg(feature = "reqwest")]
mod reqwest {
    use super::*;
    use crate::common::reqwest::client;

    #[tokio::test]
    /// GIVEN an email address
    /// WHEN creating an invitation for it
    /// THEN the created invitation is returned
    async fn creates_invitation() {
        let server = MockServer::shared();

        let invitation = client(server)
            .send(&create_invitation())
            .await
            .expect("invitation creation should succeed");

        assert!(!invitation.email.is_empty());
        assert_eq!(invitation.access_level, InvitationAccessLevel::Manager);
    }

    #[tokio::test]
    /// GIVEN pagination and ordering parameters
    /// WHEN listing invitations
    /// THEN a page of invitations is returned
    async fn lists_invitations() {
        let server = MockServer::shared();

        let page = client(server)
            .send(&list_invitations())
            .await
            .expect("invitation listing should succeed");

        assert!(!page.items.is_empty());
        assert!(page.items.iter().all(|invitation| !invitation.email.is_empty()));
    }

    #[tokio::test]
    /// GIVEN an invitation id
    /// WHEN deleting it
    /// THEN the empty response is accepted
    async fn deletes_invitation() {
        client(MockServer::shared())
            .send(&delete_invitation())
            .await
            .expect("deletion should succeed");
    }
}
