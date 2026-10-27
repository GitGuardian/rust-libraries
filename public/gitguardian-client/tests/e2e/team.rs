//! Tests of /v1/teams

use crate::common::assert_api_status;
use crate::common::fixture::{create_team, delete_team, list_teams, retrieve_team, update_team};
use gitguardian_mock::MockServer;
use http::StatusCode;

#[cfg(feature = "ureq")]
mod ureq {
    use super::*;
    use crate::common::ureq::{client, client_preferring};

    #[test]
    /// GIVEN a team name
    /// WHEN creating the team
    /// THEN the created team is returned
    fn creates_team() {
        let server = MockServer::shared();

        let team = client(server).send(&create_team()).expect("team creation should succeed");

        assert!(!team.name.is_empty());
        assert!(!team.gitguardian_url.is_empty());
    }

    #[test]
    /// GIVEN pagination and filter parameters
    /// WHEN listing teams
    /// THEN a page of teams is returned
    fn lists_teams() {
        let server = MockServer::shared();

        let page = client(server).send(&list_teams()).expect("team listing should succeed");

        assert!(!page.items.is_empty());
        assert!(page.items.iter().all(|team| !team.name.is_empty()));
    }

    #[test]
    /// GIVEN a listing whose response carries a Link header
    /// WHEN reading the page
    /// THEN the cursors are parsed out of the header
    fn reads_cursors_from_link_header() {
        let server = MockServer::shared();

        let page = client(server).send(&list_teams()).expect("team listing should succeed");

        assert!(page.next.is_some());
        assert!(page.previous.is_some());
    }

    #[test]
    /// GIVEN a mock that always reports a next cursor
    /// WHEN taking two pages from the paginated call
    /// THEN both pages are fetched
    fn paginates_teams() {
        let server = MockServer::shared();
        let client = client(server);

        let pages = client
            .paginate(list_teams())
            .take(2)
            .collect::<Result<Vec<_>, _>>()
            .expect("team pagination should succeed");

        assert_eq!(pages.len(), 2);
        assert!(pages.iter().all(|page| !page.items.is_empty()));
    }

    #[test]
    /// GIVEN a server that rejects the api key
    /// WHEN iterating the paginated call
    /// THEN the error is yielded once and iteration stops
    fn stops_paginating_after_error() {
        let server = MockServer::shared();
        let client = client_preferring(server, 401);
        let mut pages = client.paginate(list_teams());

        let error = pages.next().expect("the failing page should be yielded").unwrap_err();

        assert_api_status(&error, StatusCode::UNAUTHORIZED, "Invalid API key.");
        assert!(pages.next().is_none());
    }

    #[test]
    /// GIVEN a team id
    /// WHEN retrieving that team
    /// THEN the team is returned
    fn retrieves_team() {
        let server = MockServer::shared();

        let team = client(server).send(&retrieve_team()).expect("team retrieval should succeed");

        assert!(!team.name.is_empty());
        assert!(!team.gitguardian_url.is_empty());
    }

    #[test]
    /// GIVEN a team id and fields to change
    /// WHEN updating that team
    /// THEN the updated team is returned
    fn updates_team() {
        let team =
            client(MockServer::shared()).send(&update_team()).expect("team update should succeed");

        assert!(!team.name.is_empty());
        assert!(!team.gitguardian_url.is_empty());
    }

    #[test]
    /// GIVEN a team id
    /// WHEN deleting it
    /// THEN the empty response is accepted
    fn deletes_team() {
        client(MockServer::shared()).send(&delete_team()).expect("deletion should succeed");
    }
}

#[cfg(feature = "reqwest")]
mod reqwest {
    use super::*;
    use crate::common::reqwest::{client, client_preferring};
    use futures_util::{StreamExt, TryStreamExt};

    #[tokio::test]
    /// GIVEN a team name
    /// WHEN creating the team
    /// THEN the created team is returned
    async fn creates_team() {
        let server = MockServer::shared();

        let team = client(server).send(&create_team()).await.expect("team creation should succeed");

        assert!(!team.name.is_empty());
        assert!(!team.gitguardian_url.is_empty());
    }

    #[tokio::test]
    /// GIVEN pagination and filter parameters
    /// WHEN listing teams
    /// THEN a page of teams is returned
    async fn lists_teams() {
        let server = MockServer::shared();

        let page = client(server).send(&list_teams()).await.expect("team listing should succeed");

        assert!(!page.items.is_empty());
        assert!(page.items.iter().all(|team| !team.name.is_empty()));
    }

    #[tokio::test]
    /// GIVEN a listing whose response carries a Link header
    /// WHEN reading the page
    /// THEN the cursors are parsed out of the header
    async fn reads_cursors_from_link_header() {
        let server = MockServer::shared();

        let page = client(server).send(&list_teams()).await.expect("team listing should succeed");

        assert!(page.next.is_some());
        assert!(page.previous.is_some());
    }

    #[tokio::test]
    /// GIVEN a mock that always reports a next cursor
    /// WHEN taking two pages from the paginated call
    /// THEN both pages are fetched
    async fn paginates_teams() {
        let server = MockServer::shared();
        let client = client(server);

        let pages: Vec<_> = client
            .paginate(list_teams())
            .take(2)
            .try_collect()
            .await
            .expect("team pagination should succeed");

        assert_eq!(pages.len(), 2);
        assert!(pages.iter().all(|page| !page.items.is_empty()));
    }

    #[tokio::test]
    /// GIVEN a server that rejects the api key
    /// WHEN iterating the paginated call
    /// THEN the error is yielded once and iteration stops
    async fn stops_paginating_after_error() {
        let server = MockServer::shared();
        let client = client_preferring(server, 401);
        let mut pages = client.paginate(list_teams());

        let error = pages.next().await.expect("the failing page should be yielded").unwrap_err();

        assert_api_status(&error, StatusCode::UNAUTHORIZED, "Invalid API key.");
        assert!(pages.next().await.is_none());
    }

    #[tokio::test]
    /// GIVEN a team id
    /// WHEN retrieving that team
    /// THEN the team is returned
    async fn retrieves_team() {
        let server = MockServer::shared();

        let team =
            client(server).send(&retrieve_team()).await.expect("team retrieval should succeed");

        assert!(!team.name.is_empty());
        assert!(!team.gitguardian_url.is_empty());
    }

    #[tokio::test]
    /// GIVEN a team id and fields to change
    /// WHEN updating that team
    /// THEN the updated team is returned
    async fn updates_team() {
        let team = client(MockServer::shared())
            .send(&update_team())
            .await
            .expect("team update should succeed");

        assert!(!team.name.is_empty());
        assert!(!team.gitguardian_url.is_empty());
    }

    #[tokio::test]
    /// GIVEN a team id
    /// WHEN deleting it
    /// THEN the empty response is accepted
    async fn deletes_team() {
        client(MockServer::shared()).send(&delete_team()).await.expect("deletion should succeed");
    }
}
