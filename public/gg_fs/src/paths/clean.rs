use std::borrow::Cow;
use std::path::{Component, Path, PathBuf};

/// Resolves `.` and `..` components without accessing the filesystem.
///
/// A `..` that follows the root is dropped, and one that starts a relative path
/// is kept. An empty result is `.`.
pub fn lexical_clean(path: &Path) -> Cow<'_, Path> {
    let mut cleaned = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => match cleaned.components().next_back() {
                Some(Component::Normal(_)) => {
                    cleaned.pop();
                }
                Some(Component::RootDir) => {}
                _ => cleaned.push(".."),
            },
            other => cleaned.push(other),
        }
    }
    if cleaned.as_os_str().is_empty() {
        cleaned.push(".");
    }
    if cleaned.as_os_str() == path.as_os_str() { Cow::Borrowed(path) } else { Cow::Owned(cleaned) }
}
