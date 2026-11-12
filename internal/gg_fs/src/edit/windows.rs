use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};

use super::OP;
use crate::error::{Cause, Context, Error, Result};
use crate::file::File;
use crate::metadata::{self, Metadata};
use crate::paths::parent_dir;
use crate::sys;
use crate::write::{WriteOptions, write_with};

const LOCK_OFFSET: u64 = 1 << 62;

pub(super) fn try_lock(file: &std::fs::File) -> io::Result<bool> {
    sys::try_lock(file, LOCK_OFFSET)
}

pub(super) fn may_write_in_place(_meta: &Metadata) -> bool {
    false
}

pub(super) struct Dir {
    path: PathBuf,
}

impl Dir {
    pub(super) fn open(path: &Path) -> Result<Self> {
        Ok(Self { path: parent_dir(path).to_path_buf() })
    }

    pub(super) fn require_entry(&self, file: &File) -> Result<()> {
        let opened = sys::file_id(file).context(OP, file.path())?;
        match sys::path_id(file.path()) {
            Ok(id) if id == opened => Ok(()),
            Err(err) if err.kind() != io::ErrorKind::NotFound => {
                Err(Error::new(OP, file.path(), err))
            }
            _ => Err(Error::new(OP, file.path(), Cause::Changed)),
        }
    }

    pub(super) fn replace(
        &self,
        name: &OsStr,
        contents: &[u8],
        original: &Metadata,
        durable: bool,
        expected: Option<&File>,
    ) -> Result<()> {
        if let Some(file) = expected {
            self.require_entry(file)?;
        }
        let options = WriteOptions {
            mode: metadata::mode(original),
            break_hardlinks: true,
            durable,
            ..WriteOptions::default()
        };
        write_with(self.path.join(name), contents, &options)
    }
}
