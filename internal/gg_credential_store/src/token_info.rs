use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The `type` of a token.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenType {
    PersonalAccessToken,
    ServiceAccount,
    MachineAccessToken,
    #[serde(untagged)]
    Other(String),
}

impl From<&str> for TokenType {
    fn from(name: &str) -> Self {
        match name {
            "personal_access_token" => Self::PersonalAccessToken,
            "service_account" => Self::ServiceAccount,
            "machine_access_token" => Self::MachineAccessToken,
            other => Self::Other(other.to_owned()),
        }
    }
}

impl fmt::Display for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::PersonalAccessToken => "personal_access_token",
            Self::ServiceAccount => "service_account",
            Self::MachineAccessToken => "machine_access_token",
            Self::Other(name) => name,
        })
    }
}

/// The details of a token, as returned by the GitGuardian API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenInfo {
    pub token_type: TokenType,
    /// The expiry date of the token, if it has one.
    pub expire_at: Option<DateTime<Utc>>,
    /// The workspace the token belongs to.
    pub workspace_id: u64,
    /// The name of the token.
    pub name: String,
    /// The scopes the token grants. The store records them for a machine token
    /// only.
    pub scopes: Vec<String>,
}

#[cfg(test)]
mod tests {
    use yare::parameterized;

    use super::*;

    /// GIVEN the `type` of a token as written in a file
    /// WHEN it is parsed
    /// THEN it is the matching token type
    #[parameterized(
        personal_access_token = { "personal_access_token", TokenType::PersonalAccessToken },
        service_account = { "service_account", TokenType::ServiceAccount },
        machine_access_token = { "machine_access_token", TokenType::MachineAccessToken },
        unknown = { "future_token", TokenType::Other("future_token".to_owned()) },
    )]
    fn token_type_is_parsed_from_its_name(name: &str, expected: TokenType) {
        assert_eq!(serde_yaml_ng::from_str::<TokenType>(name).unwrap(), expected);
    }

    /// GIVEN a token type
    /// WHEN it is written
    /// THEN the output is its name
    #[parameterized(
        personal_access_token = { TokenType::PersonalAccessToken, "personal_access_token" },
        service_account = { TokenType::ServiceAccount, "service_account" },
        machine_access_token = { TokenType::MachineAccessToken, "machine_access_token" },
        unknown = { TokenType::Other("future_token".to_owned()), "future_token" },
    )]
    fn token_type_is_written_as_its_name(token_type: TokenType, expected: &str) {
        assert_eq!(serde_yaml_ng::to_string(&token_type).unwrap(), format!("{expected}\n"));
    }
}
