use std::ffi::{CStr, c_char, c_void};
use std::ptr;

use core_foundation::array::CFArray;
use core_foundation::base::{CFType, CFTypeRef, TCFType};
use core_foundation::dictionary::CFDictionary;
use core_foundation::string::CFString;
use core_foundation_sys::array::CFArrayRef;
use core_foundation_sys::base::{Boolean, OSStatus};
use core_foundation_sys::string::CFStringRef;
use security_framework_sys::base::{SecAccessRef, SecKeychainRef};
use security_framework_sys::item::{
    kSecAttrAccount, kSecAttrService, kSecClass, kSecClassGenericPassword, kSecMatchLimit,
    kSecMatchSearchList, kSecReturnData, kSecUseKeychain, kSecValueData,
};
use security_framework_sys::keychain::{
    SecKeychainCopyDomainDefault, SecKeychainGetUserInteractionAllowed,
    SecKeychainSetUserInteractionAllowed, SecPreferencesDomain,
};
use security_framework_sys::keychain_item::{SecItemAdd, SecItemCopyMatching, SecItemDelete};

use crate::Cause;

type SecTrustedApplicationRef = *mut c_void;
type SecKeychainStatus = u32;

const K_SEC_UNLOCK_STATE_STATUS: SecKeychainStatus = 1;
const ERR_SEC_ITEM_NOT_FOUND: OSStatus = -25300;
const ERR_SEC_AUTH_FAILED: OSStatus = -25293;
const ERR_SEC_INTERACTION_NOT_ALLOWED: OSStatus = -25308;

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

/// Returns the login keychain of the current user.
pub(super) fn login_keychain() -> Result<CFType, Cause> {
    let mut keychain: SecKeychainRef = ptr::null_mut();
    // SAFETY: the out-parameter is a local that receives a retained reference.
    let status = unsafe { SecKeychainCopyDomainDefault(SecPreferencesDomain::User, &mut keychain) };
    check("SecKeychainCopyDomainDefault", status)?;
    // SAFETY: a Copy or Create function returns a retained reference.
    unsafe { owned("SecKeychainCopyDomainDefault", keychain.cast_const().cast()) }
}

pub(super) fn is_unlocked(keychain: &CFType) -> Result<bool, Cause> {
    let mut bits: SecKeychainStatus = 0;
    // SAFETY: `keychain` is a live keychain reference, and the out-parameter is
    // a local.
    let status =
        unsafe { SecKeychainGetStatus(keychain.as_CFTypeRef().cast_mut().cast(), &mut bits) };
    check("SecKeychainGetStatus", status)?;
    Ok(bits & K_SEC_UNLOCK_STATE_STATUS != 0)
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

/// Returns an access list that trusts `applications`, named `label` in
/// dialogs.
pub(super) fn create_access(
    label: &CFString,
    applications: &CFArray<CFType>,
) -> Result<CFType, Cause> {
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
    unsafe { owned("SecAccessCreate", access.cast_const().cast()) }
}

pub(super) fn trusted_application(path: &CStr) -> Result<CFType, Cause> {
    let mut application: SecTrustedApplicationRef = ptr::null_mut();
    // SAFETY: `path` is NUL-terminated and live for the call, and the
    // out-parameter is a local that receives a retained reference.
    let status = unsafe { SecTrustedApplicationCreateFromPath(path.as_ptr(), &mut application) };
    check("SecTrustedApplicationCreateFromPath", status)?;
    // SAFETY: a Copy or Create function returns a retained reference.
    unsafe { owned("SecTrustedApplicationCreateFromPath", application.cast_const()) }
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
/// `reference` must be null or a reference that `operation` retained for the
/// caller.
unsafe fn owned(operation: &str, reference: CFTypeRef) -> Result<CFType, Cause> {
    if reference.is_null() {
        return Err(Cause::OsStoreUnavailable(format!("{operation} returned nothing")));
    }
    // SAFETY: `reference` is non-null, and the caller guarantees it is
    // retained.
    Ok(unsafe { CFType::wrap_under_create_rule(reference) })
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
