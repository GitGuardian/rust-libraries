use std::fs::{self, Metadata};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::error::{Cause, Context, Error, Result, ResultExt};
use crate::file::{Access, File, OpenOptions};
use crate::metadata::{self, NO_FOLLOW, metadata_if_exists};
use crate::mode::Mode;
use crate::paths::parent_dir;
use crate::read::read_capped;
use crate::temp::create_unique;

const OP: &str = "write";
const TEMP_PREFIX: &str = ".gg_fs-";
const MAX_LINK_HOPS: usize = 40;

#[cfg_attr(unix, path = "unix.rs")]
#[cfg_attr(windows, path = "windows.rs")]
mod imp;

/// Options for [`write_with`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteOptions {
    /// The mode of a new file. An existing file keeps its permission bits,
    /// without the setuid, setgid and sticky bits, except with [`Mode::Exact`],
    /// which sets them. Defaults to [`Mode::Umask`].
    pub mode: Mode,
    /// Whether to replace a file that has several hard links with a new file,
    /// leaving the other links with the old contents. When `false`, such a file
    /// is overwritten in place. The option has no effect with
    /// [`WriteOptions::in_place`]. Defaults to `false`.
    pub break_hardlinks: bool,
    /// Whether to flush the file, and then its parent directory, to disk before
    /// returning. Defaults to `false`.
    pub durable: bool,
    /// Whether to write through a symbolic link, creating its target if it does
    /// not exist. When `false`, a symbolic link at the path is an error.
    /// Defaults to `false`.
    pub follow_symlinks: bool,
    /// Whether to overwrite an existing file in place, keeping its inode, owner
    /// and mode. The write is not atomic. Defaults to `false`.
    pub in_place: bool,
    /// Whether to leave the file, and its modification time, untouched if it
    /// already has the new contents and, for [`Mode::Exact`], that mode.
    /// Defaults to `false`.
    pub skip_unchanged: bool,
}

impl Default for WriteOptions {
    /// Returns options for an atomic replace with [`Mode::Umask`] that is not
    /// durable.
    fn default() -> Self {
        Self {
            mode: Mode::Umask,
            break_hardlinks: false,
            durable: false,
            follow_symlinks: false,
            in_place: false,
            skip_unchanged: false,
        }
    }
}

impl WriteOptions {
    /// Returns options for files only the owner may access:
    /// [`Mode::Exact`]`(0o600)` from creation, and durable.
    pub fn private() -> Self {
        Self { mode: Mode::Exact(0o600), durable: true, ..Self::default() }
    }

    /// Returns options for an output file the user asked for:
    /// [`Mode::HostUmask`], following symbolic links and overwriting in place.
    pub fn output() -> Self {
        Self { mode: Mode::HostUmask, follow_symlinks: true, in_place: true, ..Self::default() }
    }
}

/// Writes a slice as the entire contents of a file.
///
/// This function will create the file if it does not exist, and will entirely
/// replace its contents if it does. The replacement is atomic. Other processes
/// observe either the old or the new contents. The new file does not keep the
/// owner, extended attributes, ACL or attributes of the old one. A file with
/// several hard links is overwritten in place instead, see
/// [`WriteOptions::break_hardlinks`].
///
/// # Errors
///
/// This function will return [`Cause::NotRegularFile`] if `path` is a symbolic
/// link, directory or other special file. It will return an error if the
/// directory is not writable or the write fails.
pub fn write(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> Result<()> {
    write_with(path, contents, &WriteOptions::default())
}

/// Writes a slice as the entire contents of a file, with options.
///
/// # Errors
///
/// This function will return an error in the situations of [`write()`], except
/// that a symbolic link is followed if `options.follow_symlinks` is set.
///
/// # Platform-specific behavior
///
/// On Windows the mode is ignored and a new file inherits the ACL of its
/// directory. Hard links are not detected, so a file is always replaced. The
/// directory is not flushed if `options.durable` is set.
pub fn write_with(
    path: impl AsRef<Path>,
    contents: impl AsRef<[u8]>,
    options: &WriteOptions,
) -> Result<()> {
    let path = path.as_ref();
    let contents = contents.as_ref();
    let target = if options.follow_symlinks { resolve(path)? } else { path.to_path_buf() };
    let existing = existing_file(&target)?;
    let unchanged = options.skip_unchanged
        && existing.as_ref().is_some_and(|metadata| {
            metadata.len() == contents.len() as u64 && imp::has_mode(metadata, options.mode)
        })
        && has_contents(&target, contents);
    let linked = !options.break_hardlinks
        && existing.as_ref().and_then(metadata::hard_links).is_some_and(|links| links > 1);
    if unchanged {
        Ok(())
    } else if existing.is_some() && (options.in_place || linked) {
        overwrite(&target, contents, options)
    } else {
        replace(&target, contents, options, existing.as_ref())
    }
}

fn resolve(path: &Path) -> Result<PathBuf> {
    let mut current = path.to_path_buf();
    for _ in 0..=MAX_LINK_HOPS {
        match metadata_if_exists(&current, &NO_FOLLOW)? {
            Some(metadata) if metadata.is_symlink() => {
                let link = fs::read_link(&current).context(OP, path)?;
                current = current.with_file_name(link);
            }
            _ => return Ok(current),
        }
    }
    let message = "too many levels of symbolic links";
    Err(Error::with_kind(OP, path, io::ErrorKind::InvalidInput, message))
}

fn existing_file(path: &Path) -> Result<Option<Metadata>> {
    match metadata_if_exists(path, &NO_FOLLOW)? {
        Some(metadata) if !metadata.is_file() => Err(Error::new(OP, path, Cause::NotRegularFile)),
        found => Ok(found),
    }
}

fn has_contents(path: &Path, contents: &[u8]) -> bool {
    let options = OpenOptions { follow_symlinks: false, ..OpenOptions::default() };
    File::open_with(path, &options)
        .and_then(|mut file| read_capped(&mut file, contents.len() as u64))
        .is_ok_and(|current| current == contents)
}

fn overwrite(path: &Path, contents: &[u8], options: &WriteOptions) -> Result<()> {
    let open =
        OpenOptions { follow_symlinks: false, access: Access::ReadWrite, ..OpenOptions::default() };
    let mut file = File::open_with(path, &open)?;
    imp::set_exact_mode(&file, options.mode).context(OP, path)?;
    file.write_all(contents).context(OP, path)?;
    file.set_len(contents.len() as u64).context(OP, path)?;
    if options.durable { file.sync_all().context(OP, path) } else { Ok(()) }
}

fn replace(
    path: &Path,
    contents: &[u8],
    options: &WriteOptions,
    existing: Option<&Metadata>,
) -> Result<()> {
    let mode = match (options.mode, existing) {
        (Mode::Umask | Mode::HostUmask, Some(metadata)) => metadata::mode(metadata),
        (mode, _) => mode,
    };
    let dir = parent_dir(path);
    let (mut file, temp) = create_unique(dir, TEMP_PREFIX, |name| {
        let temp = dir.join(name);
        let file = imp::create_temp(&temp, mode).context(OP, path)?;
        Ok((file, temp))
    })?;
    file.write_all(contents)
        .and_then(|()| imp::set_new_mode(&file, mode))
        .and_then(|()| if options.durable { file.sync_all() } else { Ok(()) })
        .and_then(|()| fs::rename(&temp, path))
        .inspect_err(|_| {
            fs::remove_file(&temp).context(OP, &temp).ok_or_log();
        })
        .context(OP, path)?;
    if options.durable { imp::sync_dir(dir) } else { Ok(()) }
}
