//! Quiet, process-local legacy Keychain adapter.
//! New in-process credential consumers must preserve this no-dialog policy.
use super::StoreError;
use core_foundation::{
    array::CFArray,
    base::{CFType, TCFType},
    boolean::CFBoolean,
    data::CFData,
    dictionary::CFDictionary,
    string::{CFString, CFStringRef},
};
use security_framework_sys::{
    item::{
        kSecAttrAccount, kSecAttrService, kSecAttrSynchronizable, kSecClass,
        kSecClassGenericPassword, kSecMatchLimit, kSecMatchSearchList, kSecReturnData,
        kSecUseKeychain, kSecValueData,
    },
    keychain::{
        SecKeychainCopyDefault, SecKeychainGetUserInteractionAllowed,
        SecKeychainSetUserInteractionAllowed,
    },
    keychain_item::{SecItemAdd, SecItemCopyMatching},
};
use std::sync::OnceLock;

// This documented Security framework constant is absent from the sys crate.
unsafe extern "C" {
    static kSecMatchLimitOne: CFStringRef;
}

fn default_keychain() -> Result<CFType, StoreError> {
    static NO_UI: OnceLock<Result<(), StoreError>> = OnceLock::new();
    (*NO_UI.get_or_init(|| {
        let mut allowed = 1;
        // SAFETY: these APIs affect only this process. Recovery is the sole
        // native Keychain owner; initialization is serialized and never undone.
        // The writable Boolean output is valid for the synchronous call.
        let quiet = unsafe {
            SecKeychainSetUserInteractionAllowed(0) == 0
                && SecKeychainGetUserInteractionAllowed(&mut allowed) == 0
                && allowed == 0
        };
        if quiet {
            Ok(())
        } else {
            Err(StoreError::Protection)
        }
    }))?;
    let mut keychain = std::ptr::null_mut();
    // SAFETY: valid output descriptor; success returns a +1 CF-owned object.
    let status = unsafe { SecKeychainCopyDefault(&mut keychain) };
    if status != 0 || keychain.is_null() {
        return Err(StoreError::Protection);
    }
    // SAFETY: successful Copy with a nonnull result transfers one ownership.
    Ok(unsafe { CFType::wrap_under_create_rule(keychain.cast()) })
}

fn attributes(name: &str) -> Vec<(CFString, CFType)> {
    // SAFETY: these are framework-owned, process-lifetime CFString constants.
    // Get-rule wrappers retain them, and each value remains an owned CF object.
    unsafe {
        vec![
            (
                CFString::wrap_under_get_rule(kSecClass),
                CFString::wrap_under_get_rule(kSecClassGenericPassword).as_CFType(),
            ),
            (
                CFString::wrap_under_get_rule(kSecAttrService),
                CFString::new("Automexia recovery").as_CFType(),
            ),
            (
                CFString::wrap_under_get_rule(kSecAttrAccount),
                CFString::new(name).as_CFType(),
            ),
            (
                CFString::wrap_under_get_rule(kSecAttrSynchronizable),
                CFBoolean::false_value().as_CFType(),
            ),
        ]
    }
}

pub(super) fn read_platform_key(name: &str) -> Result<Option<Vec<u8>>, StoreError> {
    let keychain = default_keychain()?;
    let search = CFArray::from_CFTypes(&[keychain]);
    let mut pairs = attributes(name);
    // SAFETY: static framework strings are retained under the get rule; owned
    // search-array/Boolean/string values stay alive throughout the query.
    unsafe {
        pairs.extend([
            (
                CFString::wrap_under_get_rule(kSecMatchSearchList),
                search.as_CFType(),
            ),
            (
                CFString::wrap_under_get_rule(kSecMatchLimit),
                CFString::wrap_under_get_rule(kSecMatchLimitOne).as_CFType(),
            ),
            (
                CFString::wrap_under_get_rule(kSecReturnData),
                CFBoolean::true_value().as_CFType(),
            ),
        ]);
    }
    let query = CFDictionary::from_CFType_pairs(&pairs);
    let mut result = std::ptr::null();
    // SAFETY: the dictionary owns all keys/values and the output descriptor is
    // valid. On success the Copy API transfers one CF ownership to this caller.
    let status = unsafe { SecItemCopyMatching(query.as_concrete_TypeRef(), &mut result) };
    if status == -25300 {
        return Ok(None); // errSecItemNotFound only; locked/ACL errors fail closed.
    }
    if status != 0 || result.is_null() {
        return Err(StoreError::Protection);
    }
    // SAFETY: the successful nonnull Copy result is a CF object, not necessarily
    // CFData. Adopt it generically, then check its dynamic type before access.
    let result = unsafe { CFType::wrap_under_create_rule(result) };
    let bytes = result.downcast::<CFData>().ok_or(StoreError::Protection)?;
    if bytes.len() != 32 {
        return Err(StoreError::Protection);
    }
    Ok(Some(bytes.bytes().to_vec()))
}

pub(super) fn write_platform_key(name: &str, key: &[u8]) -> Result<(), StoreError> {
    if key.len() != 32 {
        return Err(StoreError::Protection);
    }
    let keychain = default_keychain()?;
    let mut pairs = attributes(name);
    // SAFETY: static framework keys are retained, and CFData copies the bounded
    // key bytes into an owned value that survives the synchronous Add operation.
    unsafe {
        pairs.extend([
            (CFString::wrap_under_get_rule(kSecUseKeychain), keychain),
            (
                CFString::wrap_under_get_rule(kSecValueData),
                CFData::from_buffer(key).as_CFType(),
            ),
        ]);
    }
    let query = CFDictionary::from_CFType_pairs(&pairs);
    // SAFETY: the dictionary owns every value through the synchronous call. A
    // null result pointer explicitly requests no returned object. Never update
    // or delete on duplicate/ACL/locked errors, and never unlock the Keychain.
    let status = unsafe { SecItemAdd(query.as_concrete_TypeRef(), std::ptr::null_mut()) };
    if status == 0 {
        Ok(())
    } else {
        Err(StoreError::Protection)
    }
}
