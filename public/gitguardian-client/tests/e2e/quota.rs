//! Tests of /v1/quotas

use crate::common::fixture::retrieve_quotas;
use gitguardian_mock::MockServer;

#[cfg(feature = "ureq")]
mod ureq {
    use super::*;
    use crate::common::ureq::client;

    #[test]
    /// GIVEN a valid api key
    /// WHEN retrieving the quota overview
    /// THEN the nested quota content deserializes, including its date field
    fn retrieves_quotas() {
        client(MockServer::shared())
            .send(&retrieve_quotas())
            .expect("quota retrieval should succeed");
    }
}

#[cfg(feature = "reqwest")]
mod reqwest {
    use super::*;
    use crate::common::reqwest::client;

    #[tokio::test]
    /// GIVEN a valid api key
    /// WHEN retrieving the quota overview
    /// THEN the nested quota content deserializes, including its date field
    async fn retrieves_quotas() {
        client(MockServer::shared())
            .send(&retrieve_quotas())
            .await
            .expect("quota retrieval should succeed");
    }
}
