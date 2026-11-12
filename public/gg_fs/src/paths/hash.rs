use std::path::{Component, Path};

use sha2::{Digest, Sha256};

/// Returns a stable, platform-independent hash of `path`.
///
/// Paths with the same components hash equally on every platform. A `\` is a
/// separator on Windows only. This function does not access the filesystem, so
/// paths that refer to the same file through a symbolic link or a different case
/// hash differently unless they are [canonicalized](super::canonicalize()) first.
pub fn stable_hash(path: impl AsRef<Path>) -> u64 {
    let mut hasher = Sha256::new();
    for component in path.as_ref().components() {
        match component {
            Component::RootDir => hasher.update(b"/"),
            other => {
                let name = other.as_os_str();
                hasher.update(name.to_str().map_or(name.as_encoded_bytes(), str::as_bytes));
            }
        }
        hasher.update([0]);
    }
    hasher.finalize().first_chunk::<8>().map_or(0, |bytes| u64::from_be_bytes(*bytes))
}
