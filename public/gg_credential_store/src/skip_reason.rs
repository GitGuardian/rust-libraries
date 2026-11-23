use std::fmt;

use chrono::{DateTime, Utc};

/// Why a read did not return the machine token.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SkipReason {
    /// The machine token does not grant these required scopes.
    MissingScopes(Vec<String>),
    /// The machine token expired on this date.
    Expired(DateTime<Utc>),
    /// The machine token belongs to this other instance.
    OtherInstance(String),
    /// The machine token cannot be read, for the given reason.
    Unreadable(String),
}

impl fmt::Display for SkipReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingScopes(scopes) => {
                write!(f, "the machine token does not grant {}", scopes.join(", "))
            }
            Self::Expired(date) => write!(f, "the machine token expired on {date}"),
            Self::OtherInstance(instance) => write!(f, "the machine token belongs to {instance}"),
            Self::Unreadable(reason) => write!(f, "the machine token cannot be read: {reason}"),
        }
    }
}
