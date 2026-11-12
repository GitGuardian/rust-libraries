use std::io::{self, Seek, SeekFrom, Write};
use std::ops::Range;
use std::path::Path;
use std::time::Duration;

use sha2::{Digest, Sha256};

use crate::error::{Cause, Context, Error, Result};
use crate::file::{Access, File, OpenOptions};
use crate::read::{self, DEFAULT_READ_CAP};

const OP: &str = "edit";

#[cfg_attr(unix, path = "unix.rs")]
#[cfg_attr(windows, path = "windows.rs")]
mod imp;

/// A replacement of a range of bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replacement {
    /// The range of bytes to replace.
    pub range: Range<u64>,
    /// The bytes to put in its place.
    pub bytes: Vec<u8>,
}

/// A set of replacements to make to a file, computed against its contents at a
/// known hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    /// The SHA-256 of the contents the edit was computed against.
    pub expected_sha256: [u8; 32],
    /// The replacements to make. They must not overlap.
    pub replacements: Vec<Replacement>,
    /// Whether a failure part-way leaves the original intact. The file is
    /// replaced instead of modified in place, so its extended attributes are
    /// not kept. Defaults to `false`.
    pub crash_safe: bool,
    /// Whether to keep the original contents in `<file name>.bak`, replacing an
    /// existing backup. If the caller cannot give the backup the owner of the
    /// file, only the caller can read it. Defaults to `false`.
    pub backup: bool,
}

impl Edit {
    /// Creates an edit with `crash_safe` and `backup` disabled.
    pub fn new(expected_sha256: [u8; 32], replacements: Vec<Replacement>) -> Self {
        Self { expected_sha256, replacements, crash_safe: false, backup: false }
    }

    /// Creates an edit with `crash_safe` and `backup` enabled.
    pub fn paranoid(expected_sha256: [u8; 32], replacements: Vec<Replacement>) -> Self {
        Self { expected_sha256, replacements, crash_safe: true, backup: true }
    }

    /// Applies the edit to the file at `path`.
    ///
    /// The file is modified in place, or replaced if it has several hard links,
    /// `crash_safe` is set, or the caller is root. A replaced file keeps its
    /// owner, group and permission bits, but not its setuid, setgid and sticky
    /// bits or its extended attributes. Other edits of the same file fail while
    /// this one runs. Other writers are not excluded.
    ///
    /// # Errors
    ///
    /// This function will return [`Cause::Timeout`] if another edit holds the
    /// file, [`Cause::Changed`] if the file does not match `expected_sha256` or
    /// is moved or replaced during the edit, [`Cause::NotRegularFile`] if `path` is a
    /// symbolic link or other special file, [`Cause::TooLarge`] if it is larger
    /// than [`DEFAULT_READ_CAP`], and an
    /// [`InvalidInput`](std::io::ErrorKind::InvalidInput) error if a
    /// replacement overlaps another or lies beyond the end of the file. It will
    /// return a [`PermissionDenied`](std::io::ErrorKind::PermissionDenied)
    /// error if the file is replaced and its owner or group cannot be kept.
    ///
    /// # Platform-specific behavior
    ///
    /// On Windows the file is always replaced, and the replacement does not keep
    /// the owner or ACL of the original.
    pub fn apply(&self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        let mut file = File::open_with(
            path,
            &OpenOptions {
                follow_symlinks: false,
                access: if self.crash_safe { Access::Read } else { Access::ReadWrite },
                ..OpenOptions::default()
            },
        )?;
        let meta = file.metadata()?;
        if !meta.is_file() {
            return Err(Error::new(OP, path, Cause::NotRegularFile));
        }
        let dir = imp::Dir::open(path)?;
        if !imp::try_lock(&file).context(OP, path)? {
            return Err(Error::new(OP, path, Cause::Timeout(Duration::ZERO)));
        }

        let original = read::read_capped(&mut file, DEFAULT_READ_CAP)?;
        if Sha256::digest(&original)[..] != self.expected_sha256[..] {
            return Err(Error::new(OP, path, Cause::Changed));
        }
        dir.require_entry(&file)?;
        let meta = file.metadata()?;
        let edited = self.splice(&original).ok_or_else(|| {
            let message = "replacements overlap or lie beyond the end of the file";
            Error::with_kind(OP, path, io::ErrorKind::InvalidInput, message)
        })?;
        if edited == original {
            return Ok(());
        }

        let name = path.file_name().unwrap_or_default();
        if self.backup {
            let mut backup = name.to_os_string();
            backup.push(".bak");
            dir.replace(&backup, &original, &meta, self.crash_safe, None)?;
        }
        if !self.crash_safe && imp::may_write_in_place(&meta) {
            file.seek(SeekFrom::Start(0)).context(OP, path)?;
            file.write_all(&edited).context(OP, path)?;
            file.set_len(edited.len() as u64).context(OP, path)?;
            dir.require_entry(&file)
        } else {
            dir.replace(name, &edited, &meta, self.crash_safe, Some(&file))
        }
    }

    fn splice(&self, original: &[u8]) -> Option<Vec<u8>> {
        let mut replacements: Vec<&Replacement> = self.replacements.iter().collect();
        replacements.sort_by_key(|r| (r.range.start, r.range.end));

        let mut edited = Vec::with_capacity(original.len());
        let mut cursor = 0;
        for r in replacements {
            let start = usize::try_from(r.range.start).ok()?;
            let end = usize::try_from(r.range.end).ok()?;
            edited.extend_from_slice(original.get(cursor..start)?);
            original.get(start..end)?;
            edited.extend_from_slice(&r.bytes);
            cursor = end;
        }
        edited.extend_from_slice(original.get(cursor..)?);
        Some(edited)
    }
}
