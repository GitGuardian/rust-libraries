//! Tests of /v1/metadata

use crate::common::fixture::retrieve_metadata;
use gitguardian_mock::MockServer;

#[cfg(feature = "ureq")]
mod ureq {
    use super::*;
    use crate::common::ureq::client;

    #[test]
    /// GIVEN a configured client
    /// WHEN retrieving the instance metadata
    /// THEN the version and the secret scan limits are returned
    fn retrieves_metadata() {
        let metadata = client(MockServer::shared())
            .send(&retrieve_metadata())
            .expect("metadata retrieval should succeed");

        assert!(!metadata.version.is_empty());
        assert!(metadata.secret_scan_preferences.maximum_document_size > 0);
        assert!(metadata.secret_scan_preferences.maximum_documents_per_scan > 0);
        assert!(!metadata.preferences.is_empty());
        assert!(!metadata.remediation_messages.is_empty());
    }
}

#[cfg(feature = "reqwest")]
mod reqwest {
    use super::*;
    use crate::common::reqwest::client;

    #[tokio::test]
    /// GIVEN a configured client
    /// WHEN retrieving the instance metadata
    /// THEN the version and the secret scan limits are returned
    async fn retrieves_metadata() {
        let metadata = client(MockServer::shared())
            .send(&retrieve_metadata())
            .await
            .expect("metadata retrieval should succeed");

        assert!(!metadata.version.is_empty());
        assert!(metadata.secret_scan_preferences.maximum_document_size > 0);
    }
}
