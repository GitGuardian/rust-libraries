//! Tests of /v1/scan

use crate::common::assert_api_status;
use crate::common::fixture::scan;
use gitguardian_mock::MockServer;
use http::StatusCode;

#[cfg(feature = "ureq")]
mod ureq {
    use super::*;
    use crate::common::ureq::{client, client_preferring};

    #[test]
    /// GIVEN a document
    /// WHEN scanning it
    /// THEN a scan result is returned
    fn scans_document() {
        let server = MockServer::shared();

        let result = client(server).send(&scan()).expect("scan should succeed");

        assert!(!result.policies.is_empty());
        assert_eq!(result.policy_break_count as usize, result.policy_breaks.len());
    }

    #[test]
    /// GIVEN a server that rejects the api key
    /// WHEN scanning a document
    /// THEN the api error reaches the caller instead of a transport error
    fn surfaces_api_error() {
        let server = MockServer::shared();

        let error = client_preferring(server, 401).send(&scan()).unwrap_err();

        assert_api_status(&error, StatusCode::UNAUTHORIZED, "Invalid API key.");
    }
}

#[cfg(feature = "reqwest")]
mod reqwest {
    use super::*;
    use crate::common::reqwest::{client, client_preferring};

    #[tokio::test]
    /// GIVEN a document
    /// WHEN scanning it
    /// THEN a scan result is returned
    async fn scans_document() {
        let server = MockServer::shared();

        let result = client(server).send(&scan()).await.expect("scan should succeed");

        assert!(!result.policies.is_empty());
        assert_eq!(result.policy_break_count as usize, result.policy_breaks.len());
    }

    #[tokio::test]
    /// GIVEN a server that rejects the api key
    /// WHEN scanning a document
    /// THEN the api error reaches the caller instead of a transport error
    async fn surfaces_api_error() {
        let server = MockServer::shared();

        let error = client_preferring(server, 401).send(&scan()).await.unwrap_err();

        assert_api_status(&error, StatusCode::UNAUTHORIZED, "Invalid API key.");
    }
}
