//! Tests of /v1/health

use crate::common::fixture::check_health;
use gitguardian_mock::MockServer;

#[cfg(feature = "ureq")]
mod ureq {
    use super::*;
    use crate::common::ureq::client;

    #[test]
    /// GIVEN a valid api key
    /// WHEN checking the api health
    /// THEN the key status is returned
    fn checks_health() {
        let status = client(MockServer::shared())
            .send(&check_health())
            .expect("health check should succeed");

        assert!(!status.detail.is_empty());
    }
}

#[cfg(feature = "reqwest")]
mod reqwest {
    use super::*;
    use crate::common::reqwest::client;

    #[tokio::test]
    /// GIVEN a valid api key
    /// WHEN checking the api health
    /// THEN the key status is returned
    async fn checks_health() {
        let status = client(MockServer::shared())
            .send(&check_health())
            .await
            .expect("health check should succeed");

        assert!(!status.detail.is_empty());
    }
}
