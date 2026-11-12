use std::fs::FileType;
use std::path::Path;

use crate::sys;

const SPECIAL_FILE_SYSTEMS: [&str; 3] = ["devfs", "autofs", "fdesc"];

/// Returns whether `path` is a directory on `devfs`, `autofs` or a similar
/// filesystem.
pub(super) fn is_special(path: &Path, file_type: FileType) -> bool {
    file_type.is_dir()
        && sys::fs_type_name(path).is_ok_and(|name| SPECIAL_FILE_SYSTEMS.contains(&name.as_str()))
}
