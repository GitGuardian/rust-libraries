//! Tests of /v1/api_tokens

use crate::common::fixture::{
    create_api_token, list_api_tokens, retrieve_api_token, retrieve_current_api_token,
    revoke_api_token, revoke_current_api_token,
};
use gitguardian_mock::MockServer;

#[cfg(feature = "ureq")]
mod ureq {
    use super::*;
    use crate::common::ureq::client;

    #[test]
    /// GIVEN a valid api key
    /// WHEN retrieving the details of the current token
    /// THEN the token is returned
    fn retrieves_current_api_token() {
        let token = client(MockServer::shared())
            .send(&retrieve_current_api_token())
            .expect("current token retrieval should succeed");

        assert!(!token.name.is_empty());
    }

    #[test]
    /// GIVEN a token id
    /// WHEN retrieving the details of that token
    /// THEN the token is returned
    fn retrieves_api_token() {
        let token = client(MockServer::shared())
            .send(&retrieve_api_token())
            .expect("token retrieval should succeed");

        assert!(!token.name.is_empty());
    }

    #[test]
    /// GIVEN pagination and filter parameters
    /// WHEN listing the workspace tokens
    /// THEN a page of tokens is returned
    fn lists_api_tokens() {
        let page = client(MockServer::shared())
            .send(&list_api_tokens())
            .expect("token listing should succeed");

        assert!(!page.items.is_empty());
        assert!(page.items.iter().all(|token| !token.name.is_empty()));
    }

    #[test]
    /// GIVEN a name, a type, scopes and a lifetime
    /// WHEN creating a token
    /// THEN the created token and its clear text key are returned
    fn creates_api_token() {
        let created = client(MockServer::shared())
            .send(&create_api_token())
            .expect("token creation should succeed");

        assert!(!created.key.is_empty());
        assert!(!created.token.name.is_empty());
        assert!(!created.token.scopes.is_empty());
    }

    #[test]
    /// GIVEN a valid api key
    /// WHEN revoking the current token
    /// THEN the empty response is accepted
    fn revokes_current_api_token() {
        client(MockServer::shared())
            .send(&revoke_current_api_token())
            .expect("revocation should succeed");
    }

    #[test]
    /// GIVEN a token id
    /// WHEN revoking that token
    /// THEN the empty response is accepted
    fn revokes_api_token() {
        client(MockServer::shared()).send(&revoke_api_token()).expect("revocation should succeed");
    }
}

#[cfg(feature = "reqwest")]
mod reqwest {
    use super::*;
    use crate::common::reqwest::client;

    #[tokio::test]
    /// GIVEN a valid api key
    /// WHEN retrieving the details of the current token
    /// THEN the token is returned
    async fn retrieves_current_api_token() {
        let token = client(MockServer::shared())
            .send(&retrieve_current_api_token())
            .await
            .expect("current token retrieval should succeed");

        assert!(!token.name.is_empty());
    }

    #[tokio::test]
    /// GIVEN a token id
    /// WHEN retrieving the details of that token
    /// THEN the token is returned
    async fn retrieves_api_token() {
        let token = client(MockServer::shared())
            .send(&retrieve_api_token())
            .await
            .expect("token retrieval should succeed");

        assert!(!token.name.is_empty());
    }

    #[tokio::test]
    /// GIVEN pagination and filter parameters
    /// WHEN listing the workspace tokens
    /// THEN a page of tokens is returned
    async fn lists_api_tokens() {
        let page = client(MockServer::shared())
            .send(&list_api_tokens())
            .await
            .expect("token listing should succeed");

        assert!(!page.items.is_empty());
    }

    #[tokio::test]
    /// GIVEN a name, a type, scopes and a lifetime
    /// WHEN creating a token
    /// THEN the created token and its clear text key are returned
    async fn creates_api_token() {
        let created = client(MockServer::shared())
            .send(&create_api_token())
            .await
            .expect("token creation should succeed");

        assert!(!created.key.is_empty());
    }

    #[tokio::test]
    /// GIVEN a token id
    /// WHEN revoking that token
    /// THEN the empty response is accepted
    async fn revokes_api_token() {
        client(MockServer::shared())
            .send(&revoke_api_token())
            .await
            .expect("revocation should succeed");
    }
}
