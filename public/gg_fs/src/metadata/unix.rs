use std::fs::Metadata;
use std::os::unix::fs::MetadataExt;

use crate::mode::Mode;

/// Returns [`Mode::Exact`] with the permission bits of `meta`.
pub(crate) fn mode(meta: &Metadata) -> Mode {
    Mode::Exact(meta.mode() & 0o777)
}

/// Returns the number of hard links of `meta`.
pub(crate) fn hard_links(meta: &Metadata) -> Option<u64> {
    Some(meta.nlink())
}
