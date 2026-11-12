use std::path::Path;

/// Returns the directory that contains `path`, or `.` if `path` names no
/// directory.
pub(crate) fn parent_dir(path: &Path) -> &Path {
    path.parent().filter(|parent| !parent.as_os_str().is_empty()).unwrap_or(Path::new("."))
}
