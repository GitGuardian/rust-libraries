use std::ffi::OsStr;
use std::fs::{self, Metadata, Permissions};
use std::io::{self, Write};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt, fchown};
use std::path::{Path, PathBuf};

use super::OP;
use crate::error::{Cause, Context, Error, Result, ResultExt};
use crate::file::File;
use crate::paths::parent_dir;
use crate::sys;
use crate::temp::create_unique;
use crate::write::TEMP_PREFIX;

pub(super) use crate::sys::try_lock;

const PERMISSION_BITS: u32 = 0o777;
const OWNER_ONLY: u32 = 0o600;

pub(super) fn may_write_in_place(meta: &Metadata) -> bool {
    meta.nlink() == 1 && sys::geteuid() != 0
}

pub(super) struct Dir {
    handle: fs::File,
    path: PathBuf,
}

impl Dir {
    pub(super) fn open(path: &Path) -> Result<Self> {
        let path = parent_dir(path).to_path_buf();
        let handle = fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY)
            .open(&path)
            .context(OP, &path)?;
        Ok(Self { handle, path })
    }

    pub(super) fn require_entry(&self, file: &File) -> Result<()> {
        let opened = sys::file_id(file).context(OP, file.path())?;
        let name = Path::new(file.path().file_name().unwrap_or_default());
        match sys::entry_id(&self.handle, name) {
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
        let path = self.path.join(name);
        let (mut temp, temp_name) = create_unique(&self.path, TEMP_PREFIX, |temp_name| {
            let temp = self.create(OsStr::new(temp_name)).context(OP, &path)?;
            Ok((temp, temp_name.to_owned()))
        })?;
        let temp_name = OsStr::new(&temp_name);
        let is_backup = expected.is_none();
        let mut fill = || -> io::Result<()> {
            temp.write_all(contents)?;
            let created = temp.metadata()?;
            let owner = (original.uid(), original.gid());
            let kept = (created.uid(), created.gid()) == owner
                || match fchown(&temp, Some(owner.0), Some(owner.1)) {
                    Err(err) if is_backup && err.kind() == io::ErrorKind::PermissionDenied => false,
                    result => result.map(|()| true)?,
                };
            let bits = if kept { PERMISSION_BITS } else { OWNER_ONLY };
            temp.set_permissions(Permissions::from_mode(original.mode() & bits))?;
            if durable { temp.sync_all() } else { Ok(()) }
        };
        let replaced = fill()
            .context(OP, &path)
            .and_then(|()| expected.map_or(Ok(()), |file| self.require_entry(file)))
            .and_then(|()| self.rename(temp_name, name).context(OP, &path));
        if replaced.is_err() {
            self.remove(temp_name).context(OP, &self.path.join(temp_name)).ok_or_log();
        }
        replaced?;
        if durable { self.handle.sync_all().context(OP, &self.path) } else { Ok(()) }
    }

    fn create(&self, name: &OsStr) -> io::Result<fs::File> {
        let name = sys::c_path(Path::new(name))?;
        let flags =
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC;
        // SAFETY: the descriptor and the name outlive the call.
        let fd = unsafe { libc::openat(self.handle.as_raw_fd(), name.as_ptr(), flags, OWNER_ONLY) };
        if fd < 0 {
            Err(io::Error::last_os_error())
        } else {
            // SAFETY: `openat` returned a new descriptor that nothing else owns.
            Ok(fs::File::from(unsafe { OwnedFd::from_raw_fd(fd) }))
        }
    }

    fn rename(&self, from: &OsStr, to: &OsStr) -> io::Result<()> {
        let (from, to) = (sys::c_path(Path::new(from))?, sys::c_path(Path::new(to))?);
        let fd = self.handle.as_raw_fd();
        // SAFETY: the descriptor and both names outlive the call.
        if unsafe { libc::renameat(fd, from.as_ptr(), fd, to.as_ptr()) } == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    fn remove(&self, name: &OsStr) -> io::Result<()> {
        let name = sys::c_path(Path::new(name))?;
        // SAFETY: the descriptor and the name outlive the call.
        if unsafe { libc::unlinkat(self.handle.as_raw_fd(), name.as_ptr(), 0) } == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }
}
