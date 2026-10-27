//! Tests of /v1/honeytokens/with-context

use crate::common::fixture::create_honeytoken_with_context;
use gitguardian_mock::MockServer;

#[cfg(feature = "ureq")]
mod ureq {
    use super::*;
    use crate::common::ureq::client;

    #[test]
    /// GIVEN a honeytoken name, type and context hints
    /// WHEN creating the honeytoken within a context
    /// THEN the file holding the honeytoken is returned
    fn creates_honeytoken_with_context() {
        let server = MockServer::shared();

        let context = client(server)
            .send(&create_honeytoken_with_context())
            .expect("honeytoken with context creation should succeed");

        assert!(!context.content.is_empty());
        assert!(!context.filename.is_empty());
        assert!(!context.honeytoken_id.is_nil());
    }
}

#[cfg(feature = "reqwest")]
mod reqwest {
    use super::*;
    use crate::common::reqwest::client;

    #[tokio::test]
    /// GIVEN a honeytoken name, type and context hints
    /// WHEN creating the honeytoken within a context
    /// THEN the file holding the honeytoken is returned
    async fn creates_honeytoken_with_context() {
        let server = MockServer::shared();

        let context = client(server)
            .send(&create_honeytoken_with_context())
            .await
            .expect("honeytoken with context creation should succeed");

        assert!(!context.content.is_empty());
        assert!(!context.filename.is_empty());
        assert!(!context.honeytoken_id.is_nil());
    }
}
