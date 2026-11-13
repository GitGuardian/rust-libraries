use serde::Serialize;

/// Type of a source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SourceType {
    Bitbucket,
    BitbucketCloud,
    Github,
    Gitlab,
    AzureDevops,
    Slack,
    JiraCloud,
    ConfluenceCloud,
    MicrosoftTeams,
    ConfluenceDataCenter,
    JiraDataCenter,
    AwsEcr,
    AzureCr,
    GoogleArtifact,
    JfrogArtifact,
    DockerHub,
    Servicenow,
    SharepointOnline,
    SharepointOnlineDrive,
    SharepointOnlinePages,
    MicrosoftOnedrive,
    CustomSource,
}
