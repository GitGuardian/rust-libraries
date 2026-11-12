use std::borrow::Cow;
use std::path::Path;

use super::case::CaseSensitivity;
use super::clean::lexical_clean;
use super::within::strip_root;

/// Returns `path` relative to `project`, with forward slashes.
///
/// Both paths are cleaned with [`lexical_clean`] first. A path outside `project`
/// is returned in full, also with forward slashes, and `project` itself is `.`.
/// Names that are not valid UTF-8 are converted lossily.
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// use gg_fs::paths::project_relative;
///
/// let project = Path::new("/work/app");
///
/// assert_eq!(project_relative(Path::new("/work/app/src/main.rs"), project), "src/main.rs");
/// assert_eq!(project_relative(Path::new("/work/other/lib.rs"), project), "/work/other/lib.rs");
/// ```
pub fn project_relative<'a>(path: &'a Path, project: &Path) -> Cow<'a, str> {
    let project = lexical_clean(project);
    match lexical_clean(path) {
        Cow::Borrowed(path) => relative_str(path, &project),
        Cow::Owned(path) => Cow::Owned(relative_str(&path, &project).into_owned()),
    }
}

fn relative_str<'p>(path: &'p Path, project: &Path) -> Cow<'p, str> {
    let relative = strip_root(path, project, CaseSensitivity::default()).unwrap_or(path);
    let relative = relative.to_string_lossy();
    if relative.is_empty() {
        Cow::Borrowed(".")
    } else if cfg!(windows) && relative.contains('\\') {
        Cow::Owned(relative.replace('\\', "/"))
    } else {
        relative
    }
}
