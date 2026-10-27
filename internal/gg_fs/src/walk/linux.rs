use std::fs::FileType;
use std::path::Path;

use crate::sys;

const PSEUDO_FILE_SYSTEMS: [u64; 15] = [
    0x9fa0,
    0x6265_6572,
    0x1cd1,
    0x0027_e0eb,
    0x6367_7270,
    0x6462_6720,
    0x7472_6163,
    0x7363_6673,
    0xf97c_ff8c,
    0xcafe_4a11,
    0x6165_676c,
    0x6265_6570,
    0x4249_4e4d,
    0x6573_5543,
    0x1980_0202,
];

/// Returns whether `path` is a directory on a pseudo-filesystem such as `proc`
/// or `sysfs`.
pub(super) fn is_special(path: &Path, file_type: FileType) -> bool {
    file_type.is_dir()
        && sys::fs_magic(path).is_ok_and(|magic| PSEUDO_FILE_SYSTEMS.contains(&magic))
}
