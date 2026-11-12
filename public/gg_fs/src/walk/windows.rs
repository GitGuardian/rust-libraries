use std::fs::FileType;
use std::os::windows::fs::MetadataExt;
use std::path::Path;

use windows_sys::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_OFFLINE, FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS, FILE_ATTRIBUTE_RECALL_ON_OPEN,
    FILE_ATTRIBUTE_REPARSE_POINT,
};

use crate::metadata::{NO_FOLLOW, metadata_with};

const PLACEHOLDER_ATTRIBUTES: u32 =
    FILE_ATTRIBUTE_OFFLINE | FILE_ATTRIBUTE_RECALL_ON_OPEN | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS;

/// Returns whether `path` is a cloud placeholder file or a reparse point that
/// is not a symbolic link or junction.
pub(super) fn is_special(path: &Path, _file_type: FileType) -> bool {
    metadata_with(path, &NO_FOLLOW).is_ok_and(|metadata| {
        let attributes = metadata.file_attributes();
        attributes & PLACEHOLDER_ATTRIBUTES != 0
            || attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 && !metadata.is_symlink()
    })
}
