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
use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::Storage::FileSystem::{
    BY_HANDLE_FILE_INFORMATION, FILE_FLAG_BACKUP_SEMANTICS, FILE_ID_INFO, FILE_READ_ATTRIBUTES,
    FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, FileIdInfo, GetFileInformationByHandle,
    GetFileInformationByHandleEx,
};
use windows_sys::Win32::System::Com::CoTaskMemFree;
use windows_sys::Win32::System::IO::IO_STATUS_BLOCK;
use windows_sys::Win32::UI::Shell::{FOLDERID_Profile, SHGetKnownFolderPath};
use windows_sys::core::PWSTR;

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

/// Returns the profile directory of the current user.
pub(crate) fn profile_dir() -> Option<PathBuf> {
    let mut raw: PWSTR = ptr::null_mut();
    let status = unsafe { SHGetKnownFolderPath(&FOLDERID_Profile, 0, ptr::null_mut(), &mut raw) };
    let dir = (status >= 0 && !raw.is_null()).then(|| {
        let len = (0..).take_while(|&offset| unsafe { *raw.add(offset) } != 0).count();
        PathBuf::from(OsString::from_wide(unsafe { slice::from_raw_parts(raw, len) }))
    });
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
