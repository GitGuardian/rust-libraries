use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::mem::{self, MaybeUninit};
use std::os::windows::ffi::OsStringExt;
use std::os::windows::fs::OpenOptionsExt;
use std::os::windows::io::AsRawHandle;
use std::path::{Path, PathBuf};
use std::{io, ptr, slice};

use windows_sys::Wdk::Storage::FileSystem::{
    FILE_CASE_SENSITIVE_INFORMATION, FileCaseSensitiveInformation, NtQueryInformationFile,
};
use windows_sys::Win32::Foundation::{CloseHandle, ERROR_LOCK_VIOLATION, HANDLE};
use windows_sys::Win32::Security::{
    CopySid, CreateWellKnownSid, EqualSid, GetLengthSid, GetTokenInformation, PSID,
    SECURITY_MAX_SID_SIZE, TOKEN_INFORMATION_CLASS, TOKEN_QUERY, WELL_KNOWN_SID_TYPE,
};
use windows_sys::Win32::Storage::FileSystem::{
    BY_HANDLE_FILE_INFORMATION, FILE_FLAG_BACKUP_SEMANTICS, FILE_ID_INFO, FILE_READ_ATTRIBUTES,
    FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, FileIdInfo, GetFileInformationByHandle,
    GetFileInformationByHandleEx, LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY, LockFileEx,
};
use windows_sys::Win32::System::Com::CoTaskMemFree;
use windows_sys::Win32::System::IO::{IO_STATUS_BLOCK, OVERLAPPED};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows_sys::Win32::UI::Shell::SHGetKnownFolderPath;
use windows_sys::core::{GUID, PWSTR};

use super::{FileId, containing_dir};

pub(crate) const SHARE_ALL: u32 = FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE;
const CASE_SENSITIVE_DIRECTORY: u32 = 1;

/// Returns the id of `file`.
pub(crate) fn file_id(file: &File) -> io::Result<FileId> {
    let handle = file.as_raw_handle() as HANDLE;
    let mut id = MaybeUninit::<FILE_ID_INFO>::zeroed();
    let extended = unsafe {
        GetFileInformationByHandleEx(
            handle,
            FileIdInfo,
            id.as_mut_ptr().cast(),
            mem::size_of::<FILE_ID_INFO>() as u32,
        )
    };
    if extended != 0 {
        let id = unsafe { id.assume_init() };
        Ok(FileId {
            volume: id.VolumeSerialNumber,
            index: u128::from_le_bytes(id.FileId.Identifier),
        })
    } else {
        let mut info = MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::zeroed();
        if unsafe { GetFileInformationByHandle(handle, info.as_mut_ptr()) } == 0 {
            Err(io::Error::last_os_error())
        } else {
            let info = unsafe { info.assume_init() };
            Ok(FileId {
                volume: u64::from(info.dwVolumeSerialNumber),
                index: u128::from(info.nFileIndexHigh) << 32 | u128::from(info.nFileIndexLow),
            })
        }
    }
}

pub(crate) fn try_lock(file: &File, offset: u64) -> io::Result<bool> {
    // SAFETY: an all-zero `OVERLAPPED` is the documented initial state.
    let mut overlapped: OVERLAPPED = unsafe { mem::zeroed() };
    overlapped.Anonymous.Anonymous.Offset = offset as u32;
    overlapped.Anonymous.Anonymous.OffsetHigh = (offset >> 32) as u32;
    // SAFETY: `file` owns a live handle and `overlapped` outlives the call.
    let locked = unsafe {
        LockFileEx(
            file.as_raw_handle() as HANDLE,
            LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY,
            0,
            1,
            0,
            &mut overlapped,
        )
    };
    if locked != 0 {
        Ok(true)
    } else {
        let err = io::Error::last_os_error();
        if err.raw_os_error() == Some(ERROR_LOCK_VIOLATION as i32) { Ok(false) } else { Err(err) }
    }
}

/// Returns the id of the file at `path`, following symbolic links.
pub(crate) fn path_id(path: &Path) -> io::Result<FileId> {
    file_id(&open_for_query(path)?)
}

/// Returns whether the directory of `path` distinguishes names by case.
pub(crate) fn case_sensitive(path: &Path) -> io::Result<bool> {
    let dir = open_for_query(containing_dir(path)?)?;
    let mut info = FILE_CASE_SENSITIVE_INFORMATION { Flags: 0 };
    let mut status_block: IO_STATUS_BLOCK = unsafe { mem::zeroed() };
    let status = unsafe {
        NtQueryInformationFile(
            dir.as_raw_handle() as HANDLE,
            &mut status_block,
            ptr::from_mut(&mut info).cast(),
            mem::size_of::<FILE_CASE_SENSITIVE_INFORMATION>() as u32,
            FileCaseSensitiveInformation,
        )
    };
    Ok(status >= 0 && info.Flags & CASE_SENSITIVE_DIRECTORY != 0)
}

/// Returns the path of the known folder `id`.
pub(crate) fn known_folder(id: &GUID) -> Option<PathBuf> {
    let mut raw: PWSTR = ptr::null_mut();
    // SAFETY: `raw` is a valid out pointer and `id` points to a known folder id.
    let status = unsafe { SHGetKnownFolderPath(id, 0, ptr::null_mut(), &mut raw) };
    let dir = (status >= 0 && !raw.is_null()).then(|| {
        // SAFETY: on success `raw` points to a string that ends with a null
        // character.
        let len = (0..).take_while(|&offset| unsafe { *raw.add(offset) } != 0).count();
        // SAFETY: the `len` characters before the null character are initialized.
        PathBuf::from(OsString::from_wide(unsafe { slice::from_raw_parts(raw, len) }))
    });
    // SAFETY: `raw` was allocated by `SHGetKnownFolderPath` or is null.
    unsafe { CoTaskMemFree(raw.cast()) };
    dir
}

fn open_for_query(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .access_mode(FILE_READ_ATTRIBUTES)
        .share_mode(SHARE_ALL)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
}

/// Does nothing and returns `0`.
pub(crate) fn set_umask(_mask: u32) -> u32 {
    0
}

pub(crate) struct SecurityIdentifier(Vec<u8>);

impl SecurityIdentifier {
    pub(crate) fn well_known(kind: WELL_KNOWN_SID_TYPE) -> Option<Self> {
        let mut buffer = vec![0u8; SECURITY_MAX_SID_SIZE as usize];
        let mut size = buffer.len() as u32;
        let created = unsafe {
            CreateWellKnownSid(kind, ptr::null_mut(), buffer.as_mut_ptr().cast(), &mut size)
        };
        (created != 0).then_some(Self(buffer))
    }

    pub(crate) fn copy_of(sid: PSID) -> Option<Self> {
        let length = unsafe { GetLengthSid(sid) };
        let mut buffer = vec![0u8; length as usize];
        let copied =
            length != 0 && unsafe { CopySid(length, buffer.as_mut_ptr().cast(), sid) } != 0;
        copied.then_some(Self(buffer))
    }

    pub(crate) fn matches(&self, other: PSID) -> bool {
        unsafe { EqualSid(self.as_psid(), other) != 0 }
    }

    pub(crate) fn as_psid(&self) -> PSID {
        self.0.as_ptr().cast_mut().cast()
    }

    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }
}

pub(crate) struct Token(HANDLE);

impl Token {
    pub(crate) fn of_process() -> Option<Self> {
        let mut token: HANDLE = ptr::null_mut();
        let opened = unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) };
        (opened != 0).then_some(Self(token))
    }

    pub(crate) fn sid(&self, class: TOKEN_INFORMATION_CLASS) -> Option<SecurityIdentifier> {
        let mut needed = 0u32;
        unsafe { GetTokenInformation(self.0, class, ptr::null_mut(), 0, &mut needed) };
        let mut buffer = vec![0u64; (needed as usize).div_ceil(mem::size_of::<u64>())];
        let size = (buffer.len() * mem::size_of::<u64>()) as u32;
        let read = unsafe {
            GetTokenInformation(self.0, class, buffer.as_mut_ptr().cast(), size, &mut needed)
        };
        if read == 0 {
            None
        } else {
            SecurityIdentifier::copy_of(unsafe { *buffer.as_ptr().cast::<PSID>() })
        }
    }
}

impl Drop for Token {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}
