use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::FromRawHandle;
use std::path::Path;
use std::{fs, io, iter, mem, ptr};

use windows_sys::Win32::Foundation::{GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Security::{
    ACCESS_ALLOWED_ACE, ACL, ACL_REVISION, AddAccessAllowedAceEx, CONTAINER_INHERIT_ACE,
    InitializeAcl, InitializeSecurityDescriptor, OBJECT_INHERIT_ACE, PSECURITY_DESCRIPTOR,
    SE_DACL_PROTECTED, SECURITY_ATTRIBUTES, SECURITY_DESCRIPTOR, SetSecurityDescriptorControl,
    SetSecurityDescriptorDacl, TokenUser,
};
use windows_sys::Win32::Storage::FileSystem::{
    CREATE_NEW, CreateDirectoryW, CreateFileW, FILE_ALL_ACCESS, FILE_ATTRIBUTE_NORMAL,
};

use super::OP;
use crate::error::{Context, Result};
use crate::sys::{SHARE_ALL, Token};

const SECURITY_DESCRIPTOR_REVISION: u32 = 1;

struct OwnerOnly {
    descriptor: Box<SECURITY_DESCRIPTOR>,
    _acl: Vec<u32>,
}

impl OwnerOnly {
    fn for_caller() -> io::Result<Self> {
        let token = Token::of_process().ok_or_else(io::Error::last_os_error)?;
        let user = token.sid(TokenUser).ok_or_else(io::Error::last_os_error)?;
        let size = mem::size_of::<ACL>() + mem::size_of::<ACCESS_ALLOWED_ACE>()
            - mem::size_of::<u32>()
            + user.len();
        let mut acl = vec![0u32; size.div_ceil(mem::size_of::<u32>())];
        let acl_ptr = acl.as_mut_ptr().cast::<ACL>();
        let acl_size = (acl.len() * mem::size_of::<u32>()) as u32;
        check(unsafe { InitializeAcl(acl_ptr, acl_size, ACL_REVISION) })?;
        check(unsafe {
            AddAccessAllowedAceEx(
                acl_ptr,
                ACL_REVISION,
                OBJECT_INHERIT_ACE | CONTAINER_INHERIT_ACE,
                FILE_ALL_ACCESS,
                user.as_psid(),
            )
        })?;
        let mut descriptor = Box::new(unsafe { mem::zeroed::<SECURITY_DESCRIPTOR>() });
        let raw: PSECURITY_DESCRIPTOR = ptr::from_mut(&mut *descriptor).cast();
        check(unsafe { InitializeSecurityDescriptor(raw, SECURITY_DESCRIPTOR_REVISION) })?;
        check(unsafe { SetSecurityDescriptorDacl(raw, 1, acl_ptr, 0) })?;
        check(unsafe { SetSecurityDescriptorControl(raw, SE_DACL_PROTECTED, SE_DACL_PROTECTED) })?;
        Ok(Self { descriptor, _acl: acl })
    }

    fn attributes(&self) -> SECURITY_ATTRIBUTES {
        SECURITY_ATTRIBUTES {
            nLength: mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: ptr::from_ref(&*self.descriptor).cast_mut().cast(),
            bInheritHandle: 0,
        }
    }

    fn create_dir(&self, path: &Path) -> io::Result<()> {
        check(unsafe { CreateDirectoryW(wide(path).as_ptr(), &self.attributes()) })
    }

    fn create_file(&self, path: &Path) -> io::Result<fs::File> {
        let handle = unsafe {
            CreateFileW(
                wide(path).as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                SHARE_ALL,
                &self.attributes(),
                CREATE_NEW,
                FILE_ATTRIBUTE_NORMAL,
                ptr::null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            Err(io::Error::last_os_error())
        } else {
            Ok(unsafe { fs::File::from_raw_handle(handle) })
        }
    }
}

pub(super) fn create_dir(path: &Path) -> Result<()> {
    OwnerOnly::for_caller().and_then(|access| access.create_dir(path)).context(OP, path)
}

pub(super) fn create_file(path: &Path) -> io::Result<fs::File> {
    OwnerOnly::for_caller()?.create_file(path)
}

fn wide(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain(iter::once(0)).collect()
}

fn check(result: i32) -> io::Result<()> {
    if result == 0 { Err(io::Error::last_os_error()) } else { Ok(()) }
}
