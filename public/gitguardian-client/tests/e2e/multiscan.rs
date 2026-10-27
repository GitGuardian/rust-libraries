//! Tests of /v1/multiscan

use crate::common::fixture::multiscan;
use gitguardian_mock::MockServer;

#[cfg(feature = "ureq")]
mod ureq {
    use super::*;
    use crate::common::ureq::client;

    #[test]
    /// GIVEN several documents
    /// WHEN scanning them
    /// THEN one scan result is returned per document
    fn scans_documents() {
        let server = MockServer::shared();

        let results = client(server).send(&multiscan()).expect("multiscan should succeed");

        assert!(!results.is_empty());
        for result in &results {
            assert!(!result.policies.is_empty());
            assert_eq!(result.policy_break_count as usize, result.policy_breaks.len());
        }
    }
}

#[cfg(feature = "reqwest")]
mod reqwest {
    use super::*;
    use crate::common::reqwest::client;

    #[tokio::test]
    /// GIVEN several documents
    /// WHEN scanning them
    /// THEN one scan result is returned per document
    async fn scans_documents() {
        let server = MockServer::shared();

        let results = client(server).send(&multiscan()).await.expect("multiscan should succeed");

        assert!(!results.is_empty());
        for result in &results {
            assert!(!result.policies.is_empty());
            assert_eq!(result.policy_break_count as usize, result.policy_breaks.len());
        }
    }
}
