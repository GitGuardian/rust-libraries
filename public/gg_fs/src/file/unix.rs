use std::os::unix::fs::{FileTypeExt, OpenOptionsExt};
use std::path::Path;
use std::{fs, io};

use super::{OP, OpenOptions, std_options};
use crate::error::{Cause, Context, Error, Result};
use crate::sys;

pub(super) fn open(path: &Path, options: &OpenOptions) -> Result<fs::File> {
    let flags = if options.follow_symlinks {
        libc::O_NONBLOCK
    } else {
        libc::O_NONBLOCK | libc::O_NOFOLLOW
    };
    let file = std_options(options.access)
        .custom_flags(flags)
        .open(path)
        .map_err(|err| open_error(path, options, err))?;
    let file_type = file.metadata().context(OP, path)?.file_type();
    if file_type.is_fifo() {
        Err(Error::new(OP, path, Cause::NotRegularFile))
    } else if file_type.is_file() || file_type.is_dir() {
        Ok(file)
    } else {
        sys::set_blocking(&file).map(|()| file).context(OP, path)
    }
}

fn open_error(path: &Path, options: &OpenOptions, err: io::Error) -> Error {
    let is_symlink = !options.follow_symlinks && err.raw_os_error() == Some(libc::ELOOP);
    if is_symlink { Error::new(OP, path, Cause::NotRegularFile) } else { Error::new(OP, path, err) }
}
