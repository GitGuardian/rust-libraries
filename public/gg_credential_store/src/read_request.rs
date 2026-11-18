use std::fmt;

/// What a caller needs from a token in [`CredentialStore::read`](crate::CredentialStore::read).
#[derive(Clone, Default, PartialEq, Eq)]
pub struct ReadRequest {
    /// The scopes the token must grant.
    pub required_scopes: Vec<String>,
    /// Whether the read ignores the machine token and the explicit token, and
    /// returns the user token only.
    pub skip_machine_token: bool,
    /// A token the caller was given, for example in an environment variable.
    /// A read returns it in place of the user token on a device without a
    /// machine token, and ignores it otherwise. An empty token counts as
    /// absent.
    pub explicit_token: Option<String>,
}

impl fmt::Debug for ReadRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReadRequest")
            .field("required_scopes", &self.required_scopes)
            .field("skip_machine_token", &self.skip_machine_token)
            .finish_non_exhaustive()
    }
}
