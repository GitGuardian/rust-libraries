use std::ffi::c_void;
use std::fs::File;
use std::mem::{self, MaybeUninit};

use std::os::windows::io::AsRawHandle;
use std::path::Path;
use std::{io, ptr};

use windows_sys::Win32::Foundation::{CloseHandle, GENERIC_ALL, GENERIC_WRITE, HANDLE, LocalFree};
use windows_sys::Win32::Security::Authorization::{GetSecurityInfo, SE_FILE_OBJECT};
use windows_sys::Win32::Security::{
    ACCESS_ALLOWED_ACE, ACE_HEADER, ACL, ACL_SIZE_INFORMATION, AclSizeInformation, CopySid,
    CreateWellKnownSid, DACL_SECURITY_INFORMATION, EqualSid, GetAce, GetAclInformation,
    GetLengthSid, GetTokenInformation, INHERIT_ONLY_ACE, OWNER_SECURITY_INFORMATION,
    PSECURITY_DESCRIPTOR, PSID, SECURITY_MAX_SID_SIZE, TOKEN_INFORMATION_CLASS, TOKEN_QUERY,
    TokenOwner, TokenUser, WELL_KNOWN_SID_TYPE, WinBuiltinAdministratorsSid, WinLocalSystemSid,
};
use windows_sys::Win32::Storage::FileSystem::{
    DELETE, FILE_APPEND_DATA, FILE_WRITE_ATTRIBUTES, FILE_WRITE_DATA, FILE_WRITE_EA, WRITE_DAC,
    WRITE_OWNER,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

use super::Trust;
use crate::error::{Cause, Context, Error, Result, UntrustedReason};
use crate::locations::{Dir, Locations};

const OP: &str = "trust";
const ACCESS_ALLOWED_ACE_TYPE: u8 = 0;
const OTHER_ALLOWED_ACE_TYPES: [u8; 4] = [4, 5, 9, 11];
const WRITE_RIGHTS: u32 = FILE_WRITE_DATA
    | FILE_APPEND_DATA
    | FILE_WRITE_EA
    | FILE_WRITE_ATTRIBUTES
    | DELETE
    | WRITE_DAC
    | WRITE_OWNER
    | GENERIC_WRITE
    | GENERIC_ALL;

struct Sid(Vec<u8>);

impl Sid {
    fn well_known(kind: WELL_KNOWN_SID_TYPE) -> Option<Self> {
        let mut buffer = vec![0u8; SECURITY_MAX_SID_SIZE as usize];
        let mut size = buffer.len() as u32;
        let created = unsafe {
            CreateWellKnownSid(kind, ptr::null_mut(), buffer.as_mut_ptr().cast(), &mut size)
        };
        (created != 0).then_some(Self(buffer))
    }

    fn copy_of(sid: PSID) -> Option<Self> {
        let length = unsafe { GetLengthSid(sid) };
        let mut buffer = vec![0u8; length as usize];
        let copied =
            length != 0 && unsafe { CopySid(length, buffer.as_mut_ptr().cast(), sid) } != 0;
        copied.then_some(Self(buffer))
    }

    fn matches(&self, other: PSID) -> bool {
        unsafe { EqualSid(self.0.as_ptr().cast_mut().cast(), other) != 0 }
    }
}

struct Descriptor {
    raw: PSECURITY_DESCRIPTOR,
    owner: PSID,
    dacl: *mut ACL,
}

impl Descriptor {
    fn of(file: &File) -> io::Result<Self> {
        let mut owner = ptr::null_mut();
        let mut dacl = ptr::null_mut();
        let mut raw = ptr::null_mut();
        let status = unsafe {
            GetSecurityInfo(
                file.as_raw_handle() as HANDLE,
                SE_FILE_OBJECT,
                OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
                &mut owner,
                ptr::null_mut(),
                &mut dacl,
                ptr::null_mut(),
                &mut raw,
            )
        };
        if status != 0 {
            Err(io::Error::from_raw_os_error(status as i32))
        } else if owner.is_null() {
            unsafe { LocalFree(raw) };
            Err(io::Error::new(io::ErrorKind::InvalidData, "the file has no owner"))
        } else {
            Ok(Self { raw, owner, dacl })
        }
    }
}

impl Drop for Descriptor {
    fn drop(&mut self) {
        unsafe { LocalFree(self.raw) };
    }
}

struct Token(HANDLE);

impl Token {
    fn of_process() -> Option<Self> {
        let mut token: HANDLE = ptr::null_mut();
        let opened = unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) };
        (opened != 0).then_some(Self(token))
    }

    fn sid(&self, class: TOKEN_INFORMATION_CLASS) -> Option<Sid> {
        let mut needed = 0u32;
        unsafe { GetTokenInformation(self.0, class, ptr::null_mut(), 0, &mut needed) };
        let mut buffer = vec![0u64; (needed as usize).div_ceil(mem::size_of::<u64>())];
        let size = (buffer.len() * mem::size_of::<u64>()) as u32;
        let read = unsafe {
            GetTokenInformation(self.0, class, buffer.as_mut_ptr().cast(), size, &mut needed)
        };
        if read == 0 { None } else { Sid::copy_of(unsafe { *buffer.as_ptr().cast::<PSID>() }) }
    }
}

impl Drop for Token {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}

pub(super) fn verify(file: &File, path: &Path, trust: &Trust) -> Result<()> {
    if *trust == Trust::Unchecked {
        Ok(())
    } else if !file.metadata().context(OP, path)?.is_file() {
        Err(Error::new(OP, path, Cause::NotRegularFile))
    } else {
        let descriptor = Descriptor::of(file).context(OP, path)?;
        let accepted = accepted_owners(trust);
        if !accepted.iter().any(|sid| sid.matches(descriptor.owner)) {
            Err(Error::new(OP, path, Cause::Untrusted(UntrustedReason::Owner)))
        } else if grants_write_to_others(&descriptor, &accepted) {
            Err(Error::new(OP, path, Cause::Untrusted(UntrustedReason::Writable)))
        } else {
            Ok(())
        }
    }
}

fn accepted_owners(trust: &Trust) -> Vec<Sid> {
    let mut owners = Vec::new();
    if matches!(trust, Trust::Caller | Trust::RootOrHomeOwner) {
        owners.extend(caller_sids());
    }
    if matches!(trust, Trust::RootOrHomeOwner | Trust::RootOnly) {
        owners.extend(administrator_sids());
    }
    if *trust == Trust::RootOrHomeOwner {
        owners.extend(home_owner());
    }
    owners
}

fn caller_sids() -> Vec<Sid> {
    Token::of_process()
        .map(|token| [token.sid(TokenUser), token.sid(TokenOwner)].into_iter().flatten().collect())
        .unwrap_or_default()
}

fn administrator_sids() -> Vec<Sid> {
    [WinLocalSystemSid, WinBuiltinAdministratorsSid]
        .into_iter()
        .filter_map(Sid::well_known)
        .collect()
}

fn home_owner() -> Option<Sid> {
    let home = Locations::current().dir(Dir::Home).ok()?;
    let dir = crate::file::File::open(home).ok()?;
    Sid::copy_of(Descriptor::of(&dir).ok()?.owner)
}

fn grants_write_to_others(descriptor: &Descriptor, accepted: &[Sid]) -> bool {
    let administrators = administrator_sids();
    let owner = Sid::copy_of(descriptor.owner);
    let trusted: Vec<&Sid> = accepted.iter().chain(&administrators).chain(&owner).collect();
    if descriptor.dacl.is_null() {
        true
    } else {
        let mut info = MaybeUninit::<ACL_SIZE_INFORMATION>::zeroed();
        let read = unsafe {
            GetAclInformation(
                descriptor.dacl,
                info.as_mut_ptr().cast(),
                mem::size_of::<ACL_SIZE_INFORMATION>() as u32,
                AclSizeInformation,
            )
        };
        read == 0
            || (0..unsafe { info.assume_init() }.AceCount)
                .any(|index| ace_grants_write(descriptor.dacl, index, &trusted))
    }
}

fn ace_grants_write(dacl: *mut ACL, index: u32, trusted: &[&Sid]) -> bool {
    let mut ace: *mut c_void = ptr::null_mut();
    if unsafe { GetAce(dacl, index, &mut ace) } == 0 {
        true
    } else {
        let header = unsafe { &*ace.cast::<ACE_HEADER>() };
        let applies = u32::from(header.AceFlags) & INHERIT_ONLY_ACE == 0;
        if applies && OTHER_ALLOWED_ACE_TYPES.contains(&header.AceType) {
            true
        } else if applies && header.AceType == ACCESS_ALLOWED_ACE_TYPE {
            let allowed = unsafe { &*ace.cast::<ACCESS_ALLOWED_ACE>() };
            let sid: PSID = ptr::from_ref(&allowed.SidStart).cast_mut().cast();
            allowed.Mask & WRITE_RIGHTS != 0 && !trusted.iter().any(|trusted| trusted.matches(sid))
        } else {
            false
        }
    }
}
