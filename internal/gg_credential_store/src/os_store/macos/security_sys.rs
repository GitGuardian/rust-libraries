use std::ffi::{CStr, c_char, c_void};
use std::path::Path;
use std::ptr;

use core_foundation::array::CFArray;
use core_foundation::base::{CFType, CFTypeRef, TCFType, TCFTypeRef};
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::CFString;
use core_foundation::url::CFURL;
use core_foundation::{declare_TCFType, impl_CFTypeDescription, impl_TCFType};
use core_foundation_sys::array::CFArrayRef;
use core_foundation_sys::base::{Boolean, OSStatus};
use core_foundation_sys::string::CFStringRef;
use security_framework_sys::access::SecAccessGetTypeID;
use security_framework_sys::base::{SecAccessRef, SecKeychainRef};
use security_framework_sys::code_signing::{
    SecCodeCheckValidity, SecCodeCopySelf, SecCodeGetTypeID, SecCodeRef,
    SecRequirementCreateWithString, SecRequirementGetTypeID, SecRequirementRef,
    SecStaticCodeCheckValidity, SecStaticCodeCreateWithPath, SecStaticCodeGetTypeID,
    SecStaticCodeRef,
};
use security_framework_sys::item::{
    kSecAttrAccount, kSecAttrService, kSecClass, kSecClassGenericPassword, kSecMatchLimit,
    kSecMatchSearchList, kSecReturnData, kSecUseKeychain, kSecValueData,
};
use security_framework_sys::keychain::{
    SecKeychainCopyDomainDefault, SecKeychainGetTypeID, SecKeychainGetUserInteractionAllowed,
    SecKeychainOpen, SecKeychainSetUserInteractionAllowed, SecPreferencesDomain,
};
use security_framework_sys::keychain_item::{SecItemAdd, SecItemCopyMatching, SecItemDelete};

use crate::Cause;

type SecTrustedApplicationRef = *mut c_void;
type SecKeychainStatus = u32;

const SYSTEM_KEYCHAIN: &CStr = c"/Library/Keychains/System.keychain";
const K_SEC_UNLOCK_STATE_STATUS: SecKeychainStatus = 1;
const ERR_SEC_ITEM_NOT_FOUND: OSStatus = -25300;
const ERR_SEC_AUTH_FAILED: OSStatus = -25293;
const ERR_SEC_INTERACTION_NOT_ALLOWED: OSStatus = -25308;
const ERR_SEC_CS_REQ_FAILED: OSStatus = -67050;
const ERR_SEC_CS_UNSIGNED: OSStatus = -67062;

#[link(name = "Security", kind = "framework")]
#[allow(non_upper_case_globals)]
unsafe extern "C" {
    static kSecAttrAccess: CFStringRef;
    static kSecMatchLimitOne: CFStringRef;
    fn SecKeychainGetStatus(keychain: SecKeychainRef, status: *mut SecKeychainStatus) -> OSStatus;
    fn SecAccessCreate(
        descriptor: CFStringRef,
        trusted_list: CFArrayRef,
        access: *mut SecAccessRef,
    ) -> OSStatus;
    fn SecTrustedApplicationCreateFromPath(
        path: *const c_char,
        application: *mut SecTrustedApplicationRef,
    ) -> OSStatus;
}

declare_TCFType!(SecKeychain, SecKeychainRef);
impl_TCFType!(SecKeychain, SecKeychainRef, SecKeychainGetTypeID);
impl_CFTypeDescription!(SecKeychain);

impl SecKeychain {
    /// Returns the login keychain of the current user.
    pub(super) fn login() -> Result<Self, Cause> {
        let mut keychain: SecKeychainRef = ptr::null_mut();
        // SAFETY: the out-parameter is a local that receives a retained
        // reference.
        let status =
            unsafe { SecKeychainCopyDomainDefault(SecPreferencesDomain::User, &mut keychain) };
        check("SecKeychainCopyDomainDefault", status)?;
        // SAFETY: a Copy or Create function returns a retained reference.
        unsafe { owned("SecKeychainCopyDomainDefault", keychain) }
    }

    pub(super) fn system() -> Result<Self, Cause> {
        let mut keychain: SecKeychainRef = ptr::null_mut();
        // SAFETY: the path is NUL-terminated and static, and the out-parameter
        // is a local that receives a retained reference.
        let status = unsafe { SecKeychainOpen(SYSTEM_KEYCHAIN.as_ptr(), &mut keychain) };
        check("SecKeychainOpen", status)?;
        // SAFETY: `SecKeychainOpen` returns a retained reference.
        unsafe { owned("SecKeychainOpen", keychain) }
    }

    pub(super) fn is_unlocked(&self) -> Result<bool, Cause> {
        let mut bits: SecKeychainStatus = 0;
        // SAFETY: `self` is a live keychain reference, and the out-parameter is
        // a local.
        let status = unsafe { SecKeychainGetStatus(self.as_concrete_TypeRef(), &mut bits) };
        check("SecKeychainGetStatus", status)?;
        Ok(bits & K_SEC_UNLOCK_STATE_STATUS != 0)
    }
}

declare_TCFType!(SecAccess, SecAccessRef);
impl_TCFType!(SecAccess, SecAccessRef, SecAccessGetTypeID);
impl_CFTypeDescription!(SecAccess);

impl SecAccess {
    /// Returns an access list that trusts the executables at `paths`, named
    /// `label` in dialogs.
    pub(super) fn new(label: &str, paths: &[&CStr]) -> Result<Self, Cause> {
        let mut applications = Vec::with_capacity(paths.len());
        for path in paths {
            let mut application: SecTrustedApplicationRef = ptr::null_mut();
            // SAFETY: `path` is NUL-terminated and live for the call, and the
            // out-parameter is a local that receives a retained reference.
            let status =
                unsafe { SecTrustedApplicationCreateFromPath(path.as_ptr(), &mut application) };
            check("SecTrustedApplicationCreateFromPath", status)?;
            // SAFETY: a Copy or Create function returns a retained reference.
            let application: CFType =
                unsafe { owned("SecTrustedApplicationCreateFromPath", application.cast_const()) }?;
            applications.push(application);
        }
        let applications = CFArray::from_CFTypes(&applications);
        let label = CFString::new(label);
        let mut access: SecAccessRef = ptr::null_mut();
        // SAFETY: `label` and `applications` are live for the call, and the
        // out-parameter is a local that receives a retained reference.
        let status = unsafe {
            SecAccessCreate(
                label.as_concrete_TypeRef(),
                applications.as_concrete_TypeRef(),
                &mut access,
            )
        };
        check("SecAccessCreate", status)?;
        // SAFETY: a Copy or Create function returns a retained reference.
        unsafe { owned("SecAccessCreate", access) }
    }
}

declare_TCFType!(SecCode, SecCodeRef);
impl_TCFType!(SecCode, SecCodeRef, SecCodeGetTypeID);
impl_CFTypeDescription!(SecCode);

impl SecCode {
    /// Returns the code of the running program.
    pub(super) fn current() -> Result<Self, Cause> {
        let mut code: SecCodeRef = ptr::null_mut();
        // SAFETY: the out-parameter is a local that receives a retained
        // reference.
        let status = unsafe { SecCodeCopySelf(0, &mut code) };
        check("SecCodeCopySelf", status)?;
        // SAFETY: a Copy or Create function returns a retained reference.
        unsafe { owned("SecCodeCopySelf", code) }
    }

    pub(super) fn satisfies(&self, requirement: &SecRequirement) -> Result<bool, Cause> {
        // SAFETY: `self` and `requirement` are live references.
        let status = unsafe {
            SecCodeCheckValidity(self.as_concrete_TypeRef(), 0, requirement.as_concrete_TypeRef())
        };
        validity("SecCodeCheckValidity", status)
    }
}

declare_TCFType!(SecStaticCode, SecStaticCodeRef);
impl_TCFType!(SecStaticCode, SecStaticCodeRef, SecStaticCodeGetTypeID);
impl_CFTypeDescription!(SecStaticCode);

impl SecStaticCode {
    /// Returns the code of the program at `path`.
    pub(super) fn at(path: &Path) -> Result<Self, Cause> {
        let url = CFURL::from_path(path, false).ok_or_else(|| {
            Cause::OsStoreUnavailable(format!("{} is not a valid path", path.display()))
        })?;
        let mut code: SecStaticCodeRef = ptr::null_mut();
        // SAFETY: `url` is live for the call, and the out-parameter is a local
        // that receives a retained reference.
        let status =
            unsafe { SecStaticCodeCreateWithPath(url.as_concrete_TypeRef(), 0, &mut code) };
        check("SecStaticCodeCreateWithPath", status)?;
        // SAFETY: a Copy or Create function returns a retained reference.
        unsafe { owned("SecStaticCodeCreateWithPath", code) }
    }

    pub(super) fn satisfies(&self, requirement: &SecRequirement) -> Result<bool, Cause> {
        // SAFETY: `self` and `requirement` are live references.
        let status = unsafe {
            SecStaticCodeCheckValidity(
                self.as_concrete_TypeRef(),
                0,
                requirement.as_concrete_TypeRef(),
            )
        };
        validity("SecStaticCodeCheckValidity", status)
    }
}

declare_TCFType!(SecRequirement, SecRequirementRef);
impl_TCFType!(SecRequirement, SecRequirementRef, SecRequirementGetTypeID);
impl_CFTypeDescription!(SecRequirement);

impl SecRequirement {
    /// Returns the requirement `text`, written in Apple's code requirement
    /// language.
    pub(super) fn new(text: &str) -> Result<Self, Cause> {
        let text = CFString::new(text);
        let mut requirement: SecRequirementRef = ptr::null_mut();
        // SAFETY: `text` is live for the call, and the out-parameter is a local
        // that receives a retained reference.
        let status = unsafe {
            SecRequirementCreateWithString(text.as_concrete_TypeRef(), 0, &mut requirement)
        };
        check("SecRequirementCreateWithString", status)?;
        // SAFETY: a Copy or Create function returns a retained reference.
        unsafe { owned("SecRequirementCreateWithString", requirement) }
    }
}

/// Returns whether the keychain may show dialogs.
pub(super) fn user_interaction_allowed() -> Result<bool, Cause> {
    let mut allowed: Boolean = 0;
    // SAFETY: the out-parameter is a local.
    let status = unsafe { SecKeychainGetUserInteractionAllowed(&mut allowed) };
    check("SecKeychainGetUserInteractionAllowed", status)?;
    Ok(allowed != 0)
}

/// Allows or forbids keychain dialogs for the whole process.
pub(super) fn set_user_interaction_allowed(allowed: bool) -> Result<(), Cause> {
    // SAFETY: the function takes a plain value.
    let status = unsafe { SecKeychainSetUserInteractionAllowed(Boolean::from(allowed)) };
    check("SecKeychainSetUserInteractionAllowed", status)
}

/// A Security framework string used in item queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Constant {
    Class,
    ClassGenericPassword,
    AttrService,
    AttrAccount,
    AttrAccess,
    ValueData,
    UseKeychain,
    ReturnData,
    MatchLimit,
    MatchLimitOne,
    MatchSearchList,
}

impl Constant {
    pub(super) fn cf_string(self) -> CFString {
        // SAFETY: the statics are immutable strings owned by the framework.
        unsafe {
            let raw = match self {
                Self::Class => kSecClass,
                Self::ClassGenericPassword => kSecClassGenericPassword,
                Self::AttrService => kSecAttrService,
                Self::AttrAccount => kSecAttrAccount,
                Self::AttrAccess => kSecAttrAccess,
                Self::ValueData => kSecValueData,
                Self::UseKeychain => kSecUseKeychain,
                Self::ReturnData => kSecReturnData,
                Self::MatchLimit => kSecMatchLimit,
                Self::MatchLimitOne => kSecMatchLimitOne,
                Self::MatchSearchList => kSecMatchSearchList,
            };
            CFString::wrap_under_get_rule(raw)
        }
    }
}

pub(super) fn add_item(attributes: &CFDictionary<CFString, CFType>) -> Result<(), Cause> {
    // SAFETY: `attributes` is live for the call, and no result is requested.
    let status = unsafe { SecItemAdd(attributes.as_concrete_TypeRef(), ptr::null_mut()) };
    check("SecItemAdd", status)
}

pub(super) fn copy_matching(query: &CFDictionary<CFString, CFType>) -> Result<CFType, Cause> {
    let mut result: CFTypeRef = ptr::null();
    // SAFETY: `query` is live for the call, and the out-parameter is a local
    // that receives a retained reference.
    let status = unsafe { SecItemCopyMatching(query.as_concrete_TypeRef(), &mut result) };
    check("SecItemCopyMatching", status)?;
    // SAFETY: a Copy or Create function returns a retained reference.
    unsafe { owned("SecItemCopyMatching", result) }
}

pub(super) fn delete_items(query: &CFDictionary<CFString, CFType>) -> Result<(), Cause> {
    // SAFETY: `query` is live for the call.
    let status = unsafe { SecItemDelete(query.as_concrete_TypeRef()) };
    check("SecItemDelete", status)
}

fn validity(operation: &str, status: OSStatus) -> Result<bool, Cause> {
    match status {
        ERR_SEC_CS_REQ_FAILED | ERR_SEC_CS_UNSIGNED => Ok(false),
        status => check(operation, status).map(|()| true),
    }
}

fn check(operation: &str, status: OSStatus) -> Result<(), Cause> {
    match status {
        0 => Ok(()),
        ERR_SEC_ITEM_NOT_FOUND => Err(Cause::OsStoreItemMissing),
        ERR_SEC_AUTH_FAILED | ERR_SEC_INTERACTION_NOT_ALLOWED => {
            tracing::debug!("{operation} failed with status {status}");
            Err(Cause::OsStoreDenied)
        }
        _ => Err(Cause::OsStoreUnavailable(format!("{operation} failed with status {status}"))),
    }
}

/// Takes ownership of `reference`, which `operation` returned.
///
/// # Safety
///
/// `reference` must be null or a reference of type `T` that `operation`
/// retained for the caller.
unsafe fn owned<T: TCFType>(operation: &str, reference: T::Ref) -> Result<T, Cause> {
    if reference.as_void_ptr().is_null() {
        return Err(Cause::OsStoreUnavailable(format!("{operation} returned nothing")));
    }
    // SAFETY: `reference` is non-null, and the caller guarantees its type and
    // that it is retained.
    Ok(unsafe { T::wrap_under_create_rule(reference) })
}

#[cfg(test)]
mod tests {
    use yare::parameterized;

    use super::*;

    /// GIVEN a failure status returned by the Security framework
    /// WHEN it is checked
    /// THEN it maps to the matching cause
    #[parameterized(
        item_not_found = { -25300, |cause| matches!(cause, Cause::OsStoreItemMissing) },
        auth_failed = { -25293, |cause| matches!(cause, Cause::OsStoreDenied) },
        interaction_not_allowed = { -25308, |cause| matches!(cause, Cause::OsStoreDenied) },
        other = {
            -25299,
            |cause| matches!(
                cause,
                Cause::OsStoreUnavailable(message) if message == "SecItemAdd failed with status -25299"
            )
        },
    )]
    fn failure_status_maps_to_cause(status: OSStatus, expected: fn(&Cause) -> bool) {
        let cause = check("SecItemAdd", status).unwrap_err();

        assert!(expected(&cause), "{cause:?}");
    }

    /// GIVEN the success status
    /// WHEN it is checked
    /// THEN the check succeeds
    #[test]
    fn success_status_is_ok() {
        assert!(check("SecItemAdd", 0).is_ok());
    }
}
