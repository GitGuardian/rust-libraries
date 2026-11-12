use std::io::Read;
use std::path::Path;

use crate::error::{Cause, Context, Error, Result};
use crate::file::{File, OpenOptions};
use crate::trust::{self, Trust};

const OP: &str = "read";

/// The default size cap for reads, 64 MiB.
pub const DEFAULT_READ_CAP: u64 = 64 * 1024 * 1024;

const BYTE_ORDER_MARK: char = '\u{feff}';

/// Options for [`read_with`] and [`read_to_string_with`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadOptions {
    /// The largest file, in bytes, that is read. Larger files fail with
    /// [`Cause::TooLarge`]. Defaults to [`DEFAULT_READ_CAP`].
    pub cap: u64,
    /// The [`Trust`] the file must satisfy, checked on the open file. Defaults
    /// to [`Trust::Unchecked`].
    pub trust: Trust,
}

impl Default for ReadOptions {
    /// Returns options with the default cap and no trust requirement.
    fn default() -> Self {
        Self { cap: DEFAULT_READ_CAP, trust: Trust::Unchecked }
    }
}

/// Reads the entire contents of a file into a bytes vector.
///
/// The file must not be larger than [`DEFAULT_READ_CAP`].
///
/// # Errors
///
/// This function will return an error if the file cannot be opened or read, or
/// if it is larger than the cap.
pub fn read(path: impl AsRef<Path>) -> Result<Vec<u8>> {
    read_with(path, &ReadOptions::default())
}

/// Reads the entire contents of a file into a bytes vector, with options.
///
/// # Errors
///
/// This function will return an error in the situations of [`read`], and if
/// the file does not satisfy `options.trust`.
pub fn read_with(path: impl AsRef<Path>, options: &ReadOptions) -> Result<Vec<u8>> {
    let path = path.as_ref();
    let unchecked = options.trust == Trust::Unchecked;
    let open = OpenOptions { follow_symlinks: unchecked, ..OpenOptions::default() };
    let mut file = File::open_with(path, &open)?;
    if !unchecked {
        trust::verify(&file, path, &options.trust)?;
    }
    read_capped(&mut file, options.cap)
}

/// Reads the entire contents of a file into a string.
///
/// A leading UTF-8 byte order mark is removed. The cap is [`DEFAULT_READ_CAP`].
///
/// # Errors
///
/// This function will return an error in the situations of [`read`], and
/// [`Cause::NotUtf8`] if the contents are not valid UTF-8.
pub fn read_to_string(path: impl AsRef<Path>) -> Result<String> {
    read_to_string_with(path, &ReadOptions::default())
}

/// Reads the entire contents of a file into a string, with options.
///
/// # Errors
///
/// This function will return an error in the situations of [`read_with`], and
/// [`Cause::NotUtf8`] if the contents are not valid UTF-8.
pub fn read_to_string_with(path: impl AsRef<Path>, options: &ReadOptions) -> Result<String> {
    let path = path.as_ref();
    let bytes = read_with(path, options)?;
    let mut text = String::from_utf8(bytes).map_err(|err| err.utf8_error()).context(OP, path)?;
    if text.starts_with(BYTE_ORDER_MARK) {
        text.drain(..BYTE_ORDER_MARK.len_utf8());
    }
    Ok(text)
}

/// Reads the contents of a newly opened `file`, failing with
/// [`Cause::TooLarge`] if it holds more than `cap` bytes.
pub(crate) fn read_capped(file: &mut File, cap: u64) -> Result<Vec<u8>> {
    let size = file.metadata()?.len();
    if size > cap {
        Err(too_large(file, size, cap))
    } else {
        let mut contents = Vec::with_capacity(usize::try_from(size).unwrap_or_default());
        let read = (&mut *file).take(cap.saturating_add(1)).read_to_end(&mut contents);
        read.context(OP, file.path())?;
        let read_len = contents.len() as u64;
        if read_len > cap { Err(too_large(file, read_len, cap)) } else { Ok(contents) }
    }
}

fn too_large(file: &File, size: u64, cap: u64) -> Error {
    Error::new(OP, file.path(), Cause::TooLarge { size, cap })
}
