//! Tests of /v1/honeytokens/endpoint-deployments

use crate::common::fixture::{
    confirm_endpoint_deployment, create_endpoint_deployment, list_endpoint_deployments,
};
use gitguardian_mock::MockServer;

#[cfg(feature = "ureq")]
mod ureq {
    use super::*;
    use crate::common::ureq::client;

    #[test]
    /// GIVEN a machine and an OS user
    /// WHEN reconciling their honeytoken deployments
    /// THEN the desired placements are returned
    fn creates_endpoint_deployment() {
        let deployments = client(MockServer::shared())
            .send(&create_endpoint_deployment())
            .expect("reconcile should succeed");

        assert!(
            deployments.deployments.iter().all(|found| {
                found.config.as_ref().is_none_or(|config| config.filename.is_some())
            })
        );
    }

    #[test]
    /// GIVEN a machine and an OS user
    /// WHEN listing their honeytoken deployments
    /// THEN the live placements are returned
    fn lists_endpoint_deployments() {
        let deployments = client(MockServer::shared())
            .send(&list_endpoint_deployments())
            .expect("listing should succeed");

        assert!(!deployments.deployments.is_empty());
    }

    #[test]
    /// GIVEN a deployment id and a reported outcome
    /// WHEN confirming the deployment
    /// THEN the updated deployment is returned
    fn confirms_endpoint_deployment() {
        let deployment = client(MockServer::shared())
            .send(&confirm_endpoint_deployment())
            .expect("confirmation should succeed");

        assert!(deployment.status.is_some());
    }
}

#[cfg(feature = "reqwest")]
mod reqwest {
    use super::*;
    use crate::common::reqwest::client;

    #[tokio::test]
    /// GIVEN a machine and an OS user
    /// WHEN listing their honeytoken deployments
    /// THEN the live placements are returned
    async fn lists_endpoint_deployments() {
        let deployments = client(MockServer::shared())
            .send(&list_endpoint_deployments())
            .await
            .expect("listing should succeed");

        assert!(!deployments.deployments.is_empty());
    }

    #[tokio::test]
    /// GIVEN a deployment id and a reported outcome
    /// WHEN confirming the deployment
    /// THEN the updated deployment is returned
    async fn confirms_endpoint_deployment() {
        let deployment = client(MockServer::shared())
            .send(&confirm_endpoint_deployment())
            .await
            .expect("confirmation should succeed");

        assert!(deployment.status.is_some());
    }
}
