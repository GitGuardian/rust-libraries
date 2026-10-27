//! Tests of /v1/honeytokens

use crate::common::fixture::create_honeytoken;
use gitguardian_api::models::honeytoken::status::HoneytokenStatus;
use gitguardian_api::models::honeytoken::r#type::HoneytokenType;
use gitguardian_mock::MockServer;

#[cfg(feature = "ureq")]
mod ureq {
    use super::*;
    use crate::common::ureq::client;

    #[test]
    /// GIVEN a honeytoken name and type
    /// WHEN creating the honeytoken
    /// THEN the created honeytoken is returned
    fn creates_honeytoken() {
        let server = MockServer::shared();

        let honeytoken =
            client(server).send(&create_honeytoken()).expect("honeytoken creation should succeed");

        assert!(!honeytoken.id.is_nil());
        assert_eq!(honeytoken.honeytoken_type, HoneytokenType::Aws);
        assert_eq!(honeytoken.status, HoneytokenStatus::Active);
        assert!(!honeytoken.token.is_empty());
    }
}

#[cfg(feature = "reqwest")]
mod reqwest {
    use super::*;
    use crate::common::reqwest::client;

    #[tokio::test]
    /// GIVEN a honeytoken name and type
    /// WHEN creating the honeytoken
    /// THEN the created honeytoken is returned
    async fn creates_honeytoken() {
        let server = MockServer::shared();

        let honeytoken = client(server)
            .send(&create_honeytoken())
            .await
            .expect("honeytoken creation should succeed");

        assert!(!honeytoken.id.is_nil());
        assert_eq!(honeytoken.honeytoken_type, HoneytokenType::Aws);
        assert_eq!(honeytoken.status, HoneytokenStatus::Active);
        assert!(!honeytoken.token.is_empty());
    }
}
