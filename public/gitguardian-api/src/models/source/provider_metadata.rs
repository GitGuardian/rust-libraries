use serde::Deserialize;

/// Metadata from the upstream provider.
///
/// Fields depend on integration; absent concepts are represented with conservative
/// defaults (e.g. archived false).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ProviderMetadata {
    pub archived: bool,
}
