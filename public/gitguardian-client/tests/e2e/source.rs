//! Tests of /v1/sources

use crate::common::fixture::list_sources;
use gitguardian_mock::MockServer;

#[cfg(feature = "ureq")]
mod ureq {
    use super::*;
    use crate::common::ureq::client;

    #[test]
    /// GIVEN pagination and filter parameters
    /// WHEN listing sources
    /// THEN a page of sources is returned with its nested incident breakdown
    fn lists_sources() {
        let page = client(MockServer::shared())
            .send(&list_sources())
            .expect("source listing should succeed");

        assert!(!page.items.is_empty());
        assert!(page.items.iter().all(|source| !source.full_name.is_empty()));
    }

    #[test]
    /// GIVEN a mock that always reports a next cursor
    /// WHEN taking two pages from the paginated call
    /// THEN both pages are fetched
    fn paginates_sources() {
        let server = MockServer::shared();
        let client = client(server);

        let pages = client
            .paginate(list_sources())
            .take(2)
            .collect::<Result<Vec<_>, _>>()
            .expect("source pagination should succeed");

        assert_eq!(pages.len(), 2);
    }
}

#[cfg(feature = "reqwest")]
mod reqwest {
    use super::*;
    use crate::common::reqwest::client;
    use futures_util::{StreamExt, TryStreamExt};

    #[tokio::test]
    /// GIVEN pagination and filter parameters
    /// WHEN listing sources
    /// THEN a page of sources is returned with its nested incident breakdown
    async fn lists_sources() {
        let page = client(MockServer::shared())
            .send(&list_sources())
            .await
            .expect("source listing should succeed");

        assert!(!page.items.is_empty());
        assert!(page.items.iter().all(|source| !source.full_name.is_empty()));
    }

    #[tokio::test]
    /// GIVEN a mock that always reports a next cursor
    /// WHEN taking two pages from the paginated call
    /// THEN both pages are fetched
    async fn paginates_sources() {
        let server = MockServer::shared();
        let client = client(server);

        let pages: Vec<_> = client
            .paginate(list_sources())
            .take(2)
            .try_collect()
            .await
            .expect("source pagination should succeed");

        assert_eq!(pages.len(), 2);
    }
}
