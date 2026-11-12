use std::fs;
use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;

use windows_sys::Win32::Storage::FileSystem::{
    FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
};

use super::{OP, OpenOptions, std_options};
use crate::error::{Cause, Context, Error, Result};
use crate::sys::SHARE_ALL;

pub(super) fn open(path: &Path, options: &OpenOptions) -> Result<fs::File> {
    let flags = if options.follow_symlinks {
        FILE_FLAG_BACKUP_SEMANTICS
    } else {
        FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT
    };
    let file = std_options(options.access)
        .share_mode(SHARE_ALL)
        .custom_flags(flags)
        .open(path)
        .context(OP, path)?;
    let attributes = file.metadata().context(OP, path)?.file_attributes();
    if !options.follow_symlinks && attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        Err(Error::new(OP, path, Cause::NotRegularFile))
    } else {
        Ok(file)
    }
}
