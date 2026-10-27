use std::ffi::OsStr;
use std::path::{Component, Path, Prefix};

use super::case::CaseSensitivity;
use super::clean::lexical_clean;

/// Returns `true` if `path` is `root` or lies below it.
///
/// Both paths are cleaned with [`lexical_clean`] first, so a `..` cannot leave
/// `root`, and `/a/bc` is not within `/a/b`. Names are compared with the default
/// [`CaseSensitivity`].
pub fn is_within(root: impl AsRef<Path>, path: impl AsRef<Path>) -> bool {
    is_within_with(root, path, CaseSensitivity::default())
}

/// Returns `true` if `path` is `root` or lies below it, comparing names with
/// `case`.
///
/// Paths are otherwise treated as in [`is_within`].
pub fn is_within_with(
    root: impl AsRef<Path>,
    path: impl AsRef<Path>,
    case: CaseSensitivity,
) -> bool {
    strip_root(&lexical_clean(path.as_ref()), &lexical_clean(root.as_ref()), case).is_some()
}

pub(super) fn strip_root<'a>(
    path: &'a Path,
    root: &Path,
    case: CaseSensitivity,
) -> Option<&'a Path> {
    let (path, root) = (bare(path), bare(root));
    let mut rest = path.components();
    let matched = root
        .components()
        .all(|expected| rest.next().is_some_and(|actual| same(actual, expected, case)));
    let escapes = match rest.clone().next() {
        Some(Component::ParentDir) => true,
        Some(Component::RootDir | Component::Prefix(_)) => !root.has_root(),
        Some(Component::Normal(_) | Component::CurDir) | None => false,
    };
    (matched && !escapes).then_some(rest.as_path())
}

fn bare(path: &Path) -> &Path {
    if path.as_os_str() == "." { Path::new("") } else { path }
}

fn same(a: Component<'_>, b: Component<'_>, case: CaseSensitivity) -> bool {
    match (a, b) {
        (Component::Prefix(a), Component::Prefix(b)) => same_prefix(a.kind(), b.kind()),
        _ => match case {
            CaseSensitivity::Sensitive => a == b,
            CaseSensitivity::Insensitive => eq_ignore_case(a.as_os_str(), b.as_os_str()),
        },
    }
}

fn same_prefix(a: Prefix<'_>, b: Prefix<'_>) -> bool {
    match (a, b) {
        (Prefix::Disk(a) | Prefix::VerbatimDisk(a), Prefix::Disk(b) | Prefix::VerbatimDisk(b)) => {
            a.eq_ignore_ascii_case(&b)
        }
        (
            Prefix::UNC(a_server, a_share) | Prefix::VerbatimUNC(a_server, a_share),
            Prefix::UNC(b_server, b_share) | Prefix::VerbatimUNC(b_server, b_share),
        ) => eq_ignore_case(a_server, b_server) && eq_ignore_case(a_share, b_share),
        (a, b) => a == b,
    }
}

fn eq_ignore_case(a: &OsStr, b: &OsStr) -> bool {
    match (a.to_str(), b.to_str()) {
        (Some(a), Some(b)) => {
            a.chars().flat_map(char::to_lowercase).eq(b.chars().flat_map(char::to_lowercase))
        }
        _ => a == b,
    }
}
