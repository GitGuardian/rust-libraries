/// The permissions of a file or directory at creation.
///
/// # Platform-specific behavior
///
/// Modes are applied on Unix. On Windows a new file or directory inherits the
/// ACL of its parent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// `0o666` for files and `0o777` for directories, with the umask of the
    /// process applied.
    #[default]
    Umask,
    /// As [`Mode::Umask`], using the umask recorded by the first call to
    /// [`set_umask`], or the current umask if there was none.
    HostUmask,
    /// Exactly these permission bits.
    Exact(u32),
}

#[cfg(unix)]
impl Mode {
    /// Returns the permission bits to set after creation, or `None` if the
    /// bits requested at creation stand. `all` is the mode of the file type
    /// before the umask.
    pub(crate) fn explicit_bits(self, all: u32) -> Option<u32> {
        match self {
            Self::Exact(bits) => Some(bits),
            Self::HostUmask => Some(all & !crate::sys::host_umask()),
            Self::Umask => None,
        }
    }
}

/// Sets the umask of the process and returns the previous one.
///
/// The first call records the previous umask as the host umask, the one used by
/// [`Mode::HostUmask`]. Changing the umask by other means invalidates the
/// recorded value. The umask applies to every thread of the process.
///
/// # Platform-specific behavior
///
/// On Windows this function does nothing and returns `0`.
pub fn set_umask(mask: u32) -> u32 {
    crate::sys::set_umask(mask)
}
