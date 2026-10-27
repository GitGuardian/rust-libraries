use crate::common::fixture::list_detector_custom_hosts;
use gitguardian_mock::MockServer;

#[cfg(feature = "ureq")]
mod ureq {
    use super::*;
    use crate::common::ureq::client;

    #[test]
    fn lists_detector_custom_hosts() {
        let configs = client(MockServer::shared())
            .send(&list_detector_custom_hosts())
            .expect("custom host listing should succeed");

        assert!(!configs.is_empty());
        assert!(configs.iter().all(|config| !config.detector_name.is_empty()));
        assert!(
            configs
                .iter()
                .flat_map(|config| &config.custom_hosts)
                .all(|host| !host.base_url.is_empty())
        );
    }
}

#[cfg(feature = "reqwest")]
mod reqwest {
    use super::*;
    use crate::common::reqwest::client;

    #[tokio::test]
    async fn lists_detector_custom_hosts() {
        let configs = client(MockServer::shared())
            .send(&list_detector_custom_hosts())
            .await
            .expect("custom host listing should succeed");

        assert!(!configs.is_empty());
        assert!(configs.iter().all(|config| !config.detector_name.is_empty()));
    }
}
