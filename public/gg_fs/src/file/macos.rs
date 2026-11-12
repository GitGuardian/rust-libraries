use std::fs;
use std::path::Path;

use super::OpenOptions;
use crate::error::Result;
use crate::sys;

pub(super) fn open(path: &Path, options: &OpenOptions) -> Result<fs::File> {
    let file = super::unix::open(path, options)?;
    if options.no_cache {
        sys::set_no_cache(&file);
    }
    Ok(file)
}
