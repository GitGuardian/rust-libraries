//! Tests of /v1/oauth/token

use crate::common::fixture::create_oauth_token;
use gitguardian_mock::MockServer;

#[cfg(feature = "ureq")]
mod ureq {
    use super::*;
    use crate::common::ureq::client;

    #[test]
    /// GIVEN an authorization code and a PKCE verifier
    /// WHEN exchanging them for an access token
    /// THEN the issued personal access token is returned
    fn creates_oauth_token() {
        let token = client(MockServer::shared())
            .send(&create_oauth_token())
            .expect("token exchange should succeed");

        assert!(!token.access_token.is_empty());
        assert!(!token.key.is_empty());
        assert!(!token.scope.is_empty());
    }
}

#[cfg(feature = "reqwest")]
mod reqwest {
    use super::*;
    use crate::common::reqwest::client;

    #[tokio::test]
    /// GIVEN an authorization code and a PKCE verifier
    /// WHEN exchanging them for an access token
    /// THEN the issued personal access token is returned
    async fn creates_oauth_token() {
        let token = client(MockServer::shared())
            .send(&create_oauth_token())
            .await
            .expect("token exchange should succeed");

        assert!(!token.access_token.is_empty());
        assert!(!token.key.is_empty());
    }
}
