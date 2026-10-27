//! Tests of /v1/incidents/secrets

use crate::common::fixture::retrieve_secret_incident;
use gitguardian_mock::MockServer;

#[cfg(feature = "ureq")]
mod ureq {
    use super::*;
    use crate::common::ureq::client;

    #[test]
    /// GIVEN an incident id and a number of occurrences to include
    /// WHEN retrieving that incident
    /// THEN the incident and its nested occurrence tree deserialize
    fn retrieves_secret_incident() {
        let incident = client(MockServer::shared())
            .send(&retrieve_secret_incident())
            .expect("incident retrieval should succeed");

        assert!(!incident.detector.name.is_empty());
        assert!(!incident.gitguardian_url.is_empty());

        let occurrence =
            incident.occurrences.first().expect("the mock should return an occurrence");
        assert!(!occurrence.source.full_name.is_empty());
        assert!(
            occurrence
                .source
                .secret_incidents_breakdown
                .open_secret_incidents
                .severity_breakdown
                .critical
                < u32::MAX
        );
        assert!(occurrence.matches.iter().all(|found| !found.name.is_empty()));
        assert!(incident.feedback_list.iter().all(|feedback| !feedback.answers.is_empty()));
        assert!(incident.custom_tags.iter().all(|tag| !tag.key.is_empty()));
    }
}

#[cfg(feature = "reqwest")]
mod reqwest {
    use super::*;
    use crate::common::reqwest::client;

    #[tokio::test]
    /// GIVEN an incident id and a number of occurrences to include
    /// WHEN retrieving that incident
    /// THEN the incident and its nested occurrence tree deserialize
    async fn retrieves_secret_incident() {
        let incident = client(MockServer::shared())
            .send(&retrieve_secret_incident())
            .await
            .expect("incident retrieval should succeed");

        assert!(!incident.detector.name.is_empty());
        assert!(!incident.gitguardian_url.is_empty());

        let occurrence =
            incident.occurrences.first().expect("the mock should return an occurrence");
        assert!(!occurrence.source.full_name.is_empty());
    }
}
