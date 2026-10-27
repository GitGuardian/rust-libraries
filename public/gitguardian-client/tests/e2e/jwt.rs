//! Tests of /v1/auth/jwt

use crate::common::fixture::create_jwt;
use gitguardian_mock::MockServer;

#[cfg(feature = "ureq")]
mod ureq {
    use super::*;
    use crate::common::ureq::client;

    #[test]
    /// GIVEN an audience
    /// WHEN creating a jwt for it
    /// THEN a token is returned
    fn creates_jwt() {
        let server = MockServer::shared();

        let jwt = client(server).send(&create_jwt()).expect("jwt creation should succeed");

        assert!(!jwt.token.is_empty());
    }
}

#[cfg(feature = "reqwest")]
mod reqwest {
    use super::*;
    use crate::common::reqwest::client;

    #[tokio::test]
    /// GIVEN an audience
    /// WHEN creating a jwt for it
    /// THEN a token is returned
    async fn creates_jwt() {
        let server = MockServer::shared();

        let jwt = client(server).send(&create_jwt()).await.expect("jwt creation should succeed");

        assert!(!jwt.token.is_empty());
    }
}
