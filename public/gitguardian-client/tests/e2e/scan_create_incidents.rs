//! Tests of /v1/scan/create-incidents

use crate::common::fixture::scan_create_incidents;
use gitguardian_mock::MockServer;

#[cfg(feature = "ureq")]
mod ureq {
    use super::*;
    use crate::common::ureq::client;

    #[test]
    /// GIVEN a document and a custom source
    /// WHEN scanning it to create incidents on that source
    /// THEN one scan result is returned per document
    fn scans_documents_and_creates_incidents() {
        let server = MockServer::shared();

        let results = client(server)
            .send(&scan_create_incidents())
            .expect("scan and incident creation should succeed");

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
    /// GIVEN a document and a custom source
    /// WHEN scanning it to create incidents on that source
    /// THEN one scan result is returned per document
    async fn scans_documents_and_creates_incidents() {
        let server = MockServer::shared();

        let results = client(server)
            .send(&scan_create_incidents())
            .await
            .expect("scan and incident creation should succeed");

        assert!(!results.is_empty());
        for result in &results {
            assert!(!result.policies.is_empty());
            assert_eq!(result.policy_break_count as usize, result.policy_breaks.len());
        }
    }
}
