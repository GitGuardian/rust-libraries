use std::io::{self, Write};
use std::path::Path;
use std::time::{Duration, Instant};
use std::{fs, process, thread};

use crate::dir;
use crate::error::{Cause, Context, Error, Result};
use crate::paths::parent_dir;

const OP: &str = "lock";

const POLL_INTERVAL: Duration = Duration::from_millis(50);

#[cfg_attr(unix, path = "unix.rs")]
#[cfg_attr(windows, path = "windows.rs")]
mod imp;

/// An exclusive advisory lock on a file.
///
/// The lock is not re-entrant: locking a path that this process already holds
/// waits like any other holder. It is released when the value is dropped or the
/// process exits. The process id of the holder is written to the file. The lock
/// file is created if it does not exist, together with any missing parent
/// directories, which have mode `0o700`.
///
/// # Platform-specific behavior
///
/// The lock is compatible with the Python `filelock` package.
#[must_use = "the lock is released when this is dropped"]
pub struct FileLock {
    _file: fs::File,
}

impl FileLock {
    /// Acquires the lock, waiting until `timeout` has elapsed.
    ///
    /// With [`Duration::ZERO`] the lock is tried once.
    ///
    /// # Errors
    ///
    /// This function will return [`Cause::Timeout`] if
    /// the lock is still held when the timeout expires, and an error if the
    /// lock file is a symbolic link or cannot be created.
    pub fn lock(path: impl AsRef<Path>, timeout: Duration) -> Result<Self> {
        let path = path.as_ref();

        let parent = parent_dir(path);
        if !parent.is_dir() {
            dir::create_dir_all(parent)?;
        }
        let deadline = Instant::now().checked_add(timeout);
        let mut lock = Self::try_lock(path)?;
        while lock.is_none() && deadline.is_none_or(|deadline| Instant::now() < deadline) {
            let remaining =
                deadline.map(|deadline| deadline.saturating_duration_since(Instant::now()));
            thread::sleep(
                remaining.map_or(POLL_INTERVAL, |remaining| POLL_INTERVAL.min(remaining)),
            );
            lock = Self::try_lock(path)?;
        }
        lock.ok_or_else(|| Error::new(OP, path, Cause::Timeout(timeout)))
    }

    fn try_lock(path: &Path) -> Result<Option<Self>> {
        let mut locked = Self::open_locked(path)?;
        while let Some(file) = &locked
            && imp::is_unlinked(file).context(OP, path)?
        {
            locked = Self::open_locked(path)?;
        }
        match locked {
            Some(file) => {
                write_pid(&file).context(OP, path)?;
                Ok(Some(Self { _file: file }))
            }
            None => Ok(None),
        }
    }

    fn open_locked(path: &Path) -> Result<Option<fs::File>> {
        let file = imp::open(path).context(OP, path)?;
        let acquired = imp::try_lock(&file).context(OP, path)?;
        Ok(acquired.then_some(file))
    }
}

fn write_pid(mut file: &fs::File) -> io::Result<()> {
    file.set_len(0)?;
    file.write_all(process::id().to_string().as_bytes())
}
