use std::fs::Metadata;

use crate::mode::Mode;

/// Returns [`Mode::Umask`].
pub(crate) fn mode(_meta: &Metadata) -> Mode {
    Mode::Umask
}

/// Returns `None`.
pub(crate) fn hard_links(_meta: &Metadata) -> Option<u64> {
    None
}
