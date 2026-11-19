use std::fmt;

use crate::Source;

/// A token read from the store.
#[derive(Clone, PartialEq, Eq)]
pub struct Credential {
    /// The token.
    pub token: String,
    /// The origin of the token.
    pub source: Source,
}

impl fmt::Debug for Credential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Credential").field("source", &self.source).finish_non_exhaustive()
    }
}
