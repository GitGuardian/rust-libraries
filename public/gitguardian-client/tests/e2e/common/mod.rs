#![allow(dead_code)]

pub mod fixture;
#[cfg(feature = "reqwest")]
pub mod reqwest;
#[cfg(feature = "ureq")]
pub mod ureq;

use gitguardian_api::ApiConfig;
use gitguardian_api::error::ApiError;
use gitguardian_client::Error;
use gitguardian_mock::MockServer;
use http::StatusCode;

pub const API_KEY: &str = "api-key";

pub fn config(server: &MockServer) -> ApiConfig {
    ApiConfig::with_base_uri(API_KEY, &server.base_uri()).unwrap()
}

pub fn assert_api_status(error: &Error, expected: StatusCode, detail: &str) {
    match error {
        Error::Api(ApiError::Status { status, detail: actual }) => {
            assert_eq!(*status, expected);
            assert!(actual.contains(detail), "{actual}");
        }
        other => panic!("expected the api error to survive the transport, got {other:?}"),
    }
}
