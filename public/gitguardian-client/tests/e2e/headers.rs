//! Tests of caller-supplied headers

use crate::common::assert_api_status;
use crate::common::fixture::retrieve_quotas;
use gitguardian_api::ApiConfig;
use gitguardian_api::http::HeaderValue;
use gitguardian_mock::{MockServer, prefer};
use http::StatusCode;

fn config_preferring(server: &MockServer, status: u16) -> ApiConfig {
    let mut config = crate::common::config(server);
    config
        .headers_mut()
        .insert(prefer::HEADER, HeaderValue::from_str(&prefer::code(status)).unwrap());
    config
}

#[cfg(feature = "ureq")]
mod ureq {
    use super::*;
    use gitguardian_client::ureq::Client;

    #[test]
    fn sends_headers_of_the_config() {
        let server = MockServer::shared();

        let error = Client::new(config_preferring(server, 401))
            .send(&retrieve_quotas())
            .expect_err("the preferred status should be returned");

        assert_api_status(&error, StatusCode::UNAUTHORIZED, "Invalid API key");
    }

    #[test]
    fn sends_no_headers_of_an_untouched_config() {
        let server = MockServer::shared();

        Client::new(crate::common::config(server))
            .send(&retrieve_quotas())
            .expect("quota retrieval should succeed");
    }
}

#[cfg(feature = "reqwest")]
mod reqwest {
    use super::*;
    use gitguardian_client::reqwest::Client;

    #[tokio::test]
    async fn sends_headers_of_the_config() {
        let server = MockServer::shared();

        let error = Client::new(config_preferring(server, 401))
            .send(&retrieve_quotas())
            .await
            .expect_err("the preferred status should be returned");

        assert_api_status(&error, StatusCode::UNAUTHORIZED, "Invalid API key");
    }

    #[tokio::test]
    async fn sends_no_headers_of_an_untouched_config() {
        let server = MockServer::shared();

        Client::new(crate::common::config(server))
            .send(&retrieve_quotas())
            .await
            .expect("quota retrieval should succeed");
    }
}
