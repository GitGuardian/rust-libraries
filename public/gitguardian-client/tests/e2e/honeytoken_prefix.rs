//! Tests of /v1/honeytokens/prefixes

use crate::common::fixture::check_honeytoken_prefixes;
use gitguardian_mock::MockServer;

#[cfg(feature = "ureq")]
mod ureq {
    use super::*;
    use crate::common::ureq::client;

    #[test]
    /// GIVEN a list of HMSL hash prefixes
    /// WHEN looking them up
    /// THEN the matching honeytoken hints are returned
    fn checks_honeytoken_prefixes() {
        let matches = client(MockServer::shared())
            .send(&check_honeytoken_prefixes())
            .expect("prefix lookup should succeed");

        assert!(!matches.matches.is_empty());
        assert!(matches.matches.iter().all(|found| !found.hint.is_empty()));
    }
}

#[cfg(feature = "reqwest")]
mod reqwest {
    use super::*;
    use crate::common::reqwest::client;

    #[tokio::test]
    /// GIVEN a list of HMSL hash prefixes
    /// WHEN looking them up
    /// THEN the matching honeytoken hints are returned
    async fn checks_honeytoken_prefixes() {
        let matches = client(MockServer::shared())
            .send(&check_honeytoken_prefixes())
            .await
            .expect("prefix lookup should succeed");

        assert!(!matches.matches.is_empty());
        assert!(matches.matches.iter().all(|found| !found.hint.is_empty()));
    }
}
