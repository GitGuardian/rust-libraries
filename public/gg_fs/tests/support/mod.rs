#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

pub struct Scratch {
    _dir: TempDir,
    root: PathBuf,
}

impl Scratch {
    pub fn new() -> Self {
        let dir = TempDir::new().unwrap();
        let root = gg_fs::paths::canonicalize(dir.path()).unwrap();
        Self { _dir: dir, root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    pub fn write(&self, relative: &str, contents: impl AsRef<[u8]>) -> PathBuf {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, contents).unwrap();
        path
    }

    pub fn dir(&self, relative: &str) -> PathBuf {
        let path = self.path(relative);
        fs::create_dir_all(&path).unwrap();
        path
    }

    pub fn names(&self, relative: &str) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(self.path(relative))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
}

#[cfg(unix)]
pub fn hold_lock(path: &Path) -> fs::File {
    use std::os::fd::AsRawFd;
    let file = fs::File::open(path).unwrap();
    assert_eq!(unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) }, 0);
    file
}

#[cfg(windows)]
pub fn hold_lock(path: &Path) -> fs::File {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY, LockFileEx,
    };
    use windows_sys::Win32::System::IO::OVERLAPPED;
    let file = fs::File::open(path).unwrap();
    let mut overlapped: OVERLAPPED = unsafe { std::mem::zeroed() };
    let locked = unsafe {
        LockFileEx(
            file.as_raw_handle() as _,
            LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY,
            0,
            u32::MAX,
            u32::MAX,
            &mut overlapped,
        )
    };
    assert_ne!(locked, 0);
    file
}

#[cfg(unix)]
pub mod unix {
    use std::fs;
    use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
    use std::path::Path;

    pub fn set_mode(path: &Path, mode: u32) {
        fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
    }

    pub fn mode_of(path: &Path) -> u32 {
        fs::metadata(path).unwrap().permissions().mode() & 0o7777
    }

    pub fn inode_of(path: &Path) -> u64 {
        fs::metadata(path).unwrap().ino()
    }

    pub fn link_count(path: &Path) -> u64 {
        fs::metadata(path).unwrap().nlink()
    }

    pub fn link(target: &Path, link: &Path) {
        symlink(target, link).unwrap();
    }

    pub fn make_fifo(path: &Path) {
        let c_path = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(c_path.as_ptr(), 0o600) }, 0);
    }

    pub fn euid() -> u32 {
        unsafe { libc::geteuid() }
    }

    pub fn is_root() -> bool {
        euid() == 0
    }

    pub fn owner_of(path: &Path) -> (u32, u32) {
        let metadata = fs::symlink_metadata(path).unwrap();
        (metadata.uid(), metadata.gid())
    }

    pub fn chown(path: &Path, uid: u32, gid: u32) {
        std::os::unix::fs::chown(path, Some(uid), Some(gid)).unwrap();
    }

    pub fn other_group(excluded: u32) -> Option<u32> {
        let count = unsafe { libc::getgroups(0, std::ptr::null_mut()) };
        let mut groups = vec![0 as libc::gid_t; usize::try_from(count).unwrap()];
        let count = unsafe { libc::getgroups(count, groups.as_mut_ptr()) };
        groups.truncate(usize::try_from(count).unwrap());
        groups.into_iter().find(|gid| *gid != excluded)
    }
}

#[cfg(windows)]
pub mod windows {
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use std::{iter, mem, ptr, slice};

    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, LocalFree};
    use windows_sys::Win32::Security::Authorization::{
        ConvertSidToStringSidW, GetNamedSecurityInfoW, SE_FILE_OBJECT,
    };
    use windows_sys::Win32::Security::{
        ACCESS_ALLOWED_ACE, ACE_HEADER, ACL, ACL_SIZE_INFORMATION, AclSizeInformation,
        DACL_SECURITY_INFORMATION, GetAce, GetAclInformation, GetSecurityDescriptorControl,
        GetTokenInformation, PSID, SE_DACL_PROTECTED, TOKEN_QUERY, TOKEN_USER, TokenUser,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    const ACCESS_ALLOWED_ACE_TYPE: u8 = 0;
    const ACCESS_DENIED_ACE_TYPE: u8 = 1;

    #[derive(Debug)]
    pub struct AccessList {
        pub protected: bool,
        pub entries: Vec<String>,
    }

    pub fn dacl_of(path: &Path) -> AccessList {
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(iter::once(0)).collect();
        let mut dacl: *mut ACL = ptr::null_mut();
        let mut descriptor = ptr::null_mut();
        let status = unsafe {
            GetNamedSecurityInfoW(
                wide.as_ptr(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                ptr::null_mut(),
                ptr::null_mut(),
                &mut dacl,
                ptr::null_mut(),
                &mut descriptor,
            )
        };
        assert_eq!(status, 0, "{}", path.display());
        let mut control = 0u16;
        let mut revision = 0u32;
        assert_ne!(
            unsafe { GetSecurityDescriptorControl(descriptor, &mut control, &mut revision) },
            0
        );
        assert!(!dacl.is_null(), "{} has no access list", path.display());
        let mut info: ACL_SIZE_INFORMATION = unsafe { mem::zeroed() };
        assert_ne!(
            unsafe {
                GetAclInformation(
                    dacl,
                    ptr::from_mut(&mut info).cast(),
                    mem::size_of::<ACL_SIZE_INFORMATION>() as u32,
                    AclSizeInformation,
                )
            },
            0
        );
        let entries = (0..info.AceCount).map(|index| entry(dacl, index)).collect();
        unsafe { LocalFree(descriptor) };
        AccessList { protected: control & SE_DACL_PROTECTED != 0, entries }
    }

    fn entry(dacl: *mut ACL, index: u32) -> String {
        let mut ace = ptr::null_mut();
        assert_ne!(unsafe { GetAce(dacl, index, &mut ace) }, 0);
        let header = unsafe { &*ace.cast::<ACE_HEADER>() };
        if [ACCESS_ALLOWED_ACE_TYPE, ACCESS_DENIED_ACE_TYPE].contains(&header.AceType) {
            let allowed = unsafe { &*ace.cast::<ACCESS_ALLOWED_ACE>() };
            sid_string(ptr::from_ref(&allowed.SidStart).cast_mut().cast())
        } else {
            format!("type {}", header.AceType)
        }
    }

    pub fn current_user() -> String {
        let mut token: HANDLE = ptr::null_mut();
        assert_ne!(unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }, 0);
        let mut needed = 0u32;
        unsafe { GetTokenInformation(token, TokenUser, ptr::null_mut(), 0, &mut needed) };
        let mut buffer = vec![0u64; (needed as usize).div_ceil(mem::size_of::<u64>())];
        let size = (buffer.len() * mem::size_of::<u64>()) as u32;
        assert_ne!(
            unsafe {
                GetTokenInformation(token, TokenUser, buffer.as_mut_ptr().cast(), size, &mut needed)
            },
            0
        );
        let user = sid_string(unsafe { &*buffer.as_ptr().cast::<TOKEN_USER>() }.User.Sid);
        unsafe { CloseHandle(token) };
        user
    }

    fn sid_string(sid: PSID) -> String {
        let mut raw = ptr::null_mut();
        assert_ne!(unsafe { ConvertSidToStringSidW(sid, &mut raw) }, 0);
        let len = (0..).take_while(|&offset| unsafe { *raw.add(offset) } != 0).count();
        let text = String::from_utf16(unsafe { slice::from_raw_parts(raw, len) }).unwrap();
        unsafe { LocalFree(raw.cast()) };
        text
    }
}
