//! Tests of /v1/teams/{team_id}/sources

use crate::common::assert_api_status;
use crate::common::fixture::{list_team_sources, update_team_sources};
use gitguardian_mock::MockServer;
use http::StatusCode;

#[cfg(feature = "ureq")]
mod ureq {
    use super::*;
    use crate::common::ureq::{client, client_preferring};

    #[test]
    /// GIVEN sources to add to a team perimeter
    /// WHEN updating the perimeter
    /// THEN the empty response is accepted
    fn updates_team_sources() {
        client(MockServer::shared())
            .send(&update_team_sources())
            .expect("team perimeter update should succeed");
    }

    #[test]
    /// GIVEN a server that rejects the api key
    /// WHEN updating the perimeter
    /// THEN the api error reaches the caller instead of a transport error
    fn surfaces_api_error() {
        let server = MockServer::shared();

        let error = client_preferring(server, 401).send(&update_team_sources()).unwrap_err();

        assert_api_status(&error, StatusCode::UNAUTHORIZED, "Invalid API key.");
    }

    #[test]
    /// GIVEN a team id and filter parameters
    /// WHEN listing the sources of the team perimeter
    /// THEN a page of sources is returned
    fn lists_team_sources() {
        let page = client(MockServer::shared())
            .send(&list_team_sources())
            .expect("team source listing should succeed");

        assert!(!page.items.is_empty());
        assert!(page.items.iter().all(|source| !source.full_name.is_empty()));
    }
}

#[cfg(feature = "reqwest")]
mod reqwest {
    use super::*;
    use crate::common::reqwest::{client, client_preferring};

    #[tokio::test]
    /// GIVEN sources to add to a team perimeter
    /// WHEN updating the perimeter
    /// THEN the empty response is accepted
    async fn updates_team_sources() {
        client(MockServer::shared())
            .send(&update_team_sources())
            .await
            .expect("team perimeter update should succeed");
    }

    #[tokio::test]
    /// GIVEN a server that rejects the api key
    /// WHEN updating the perimeter
    /// THEN the api error reaches the caller instead of a transport error
    async fn surfaces_api_error() {
        let server = MockServer::shared();

        let error = client_preferring(server, 401).send(&update_team_sources()).await.unwrap_err();

        assert_api_status(&error, StatusCode::UNAUTHORIZED, "Invalid API key.");
    }

    #[tokio::test]
    /// GIVEN a team id and filter parameters
    /// WHEN listing the sources of the team perimeter
    /// THEN a page of sources is returned
    async fn lists_team_sources() {
        let page = client(MockServer::shared())
            .send(&list_team_sources())
            .await
            .expect("team source listing should succeed");

        assert!(!page.items.is_empty());
        assert!(page.items.iter().all(|source| !source.full_name.is_empty()));
    }
}
