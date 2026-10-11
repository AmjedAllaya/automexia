// set_locale_environment and nsstring_as_str were originally taken from alacritty
// https://github.com/alacritty/alacritty/blob/94ede16ee4af8869fd6415b3530c7e12c8681578/alacritty/src/macos/locale.rs#L50
// which is licensed under Apache 2.0 license.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::{env, slice, str};

use libc::{setlocale, LC_ALL, LC_CTYPE};
use objc::runtime::{Class, Object, BOOL, YES};
use objc::{msg_send, sel, sel_impl};
use tracing::debug;
const FALLBACK_LOCALE: &str = "UTF-8";

pub fn set_locale_environment() {
    let env_locale_c = CString::new("").unwrap();
    let env_locale_ptr = unsafe { setlocale(LC_ALL, env_locale_c.as_ptr()) };
    if !env_locale_ptr.is_null() {
        let env_locale = unsafe { CStr::from_ptr(env_locale_ptr).to_string_lossy() };

        // Assume `C` locale means unchanged, since it is the default anyways.
        if env_locale != "C" {
            debug!("Using environment locale: {}", env_locale);
            return;
        }
    }

    let system_locale = system_locale();
    let accepted = system_locale.as_deref().and_then(|locale| {
        let value = CString::new(locale).ok()?;
        // SAFETY: locale initialization runs before application worker startup;
        // value is a live NUL-terminated string for this synchronous native call.
        (!unsafe { setlocale(LC_ALL, value.as_ptr()) }.is_null()).then_some(locale)
    });
    if let Some(locale) = accepted {
        env::set_var("LC_ALL", locale);
    } else {
        debug!("Using UTF-8 fallback locale");
        let fallback = c"UTF-8";
        // SAFETY: static valid locale string; initialization is single-threaded.
        unsafe { setlocale(LC_CTYPE, fallback.as_ptr()) };
        env::set_var("LC_CTYPE", FALLBACK_LOCALE);
    }
}

/// Determine system locale based on language and country code.
fn system_locale() -> Option<String> {
    objc::rc::autoreleasepool(|| {
        // SAFETY: Foundation getters return borrowed objects retained for this
        // autorelease pool. Copy their text before draining; never release a
        // class object or a +0 getter result that we did not retain.
        unsafe {
            let class = Class::get("NSLocale")?;
            let locale: *const Object = msg_send![class, currentLocale];
            if locale.is_null() {
                return None;
            }
            let language_supported: BOOL =
                msg_send![locale, respondsToSelector: sel!(languageCode)];
            let country_supported: BOOL =
                msg_send![locale, respondsToSelector: sel!(countryCode)];
            if language_supported == YES && country_supported == YES {
                let language: *const Object = msg_send![locale, languageCode];
                let country: *const Object = msg_send![locale, countryCode];
                let language = nsstring_owned(language)?;
                let country = nsstring_owned(country).unwrap_or_default();
                Some(if country.is_empty() {
                    format!("{language}.UTF-8")
                } else {
                    format!("{language}_{country}.UTF-8")
                })
            } else {
                let identifier: *const Object = msg_send![locale, localeIdentifier];
                nsstring_owned(identifier).map(|value| value + ".UTF-8")
            }
        }
    })
}

const UTF8_ENCODING: usize = 4;
const MAX_LOCALE_BYTES: usize = 256;

/// Copy a nullable Foundation locale string while its owner is still alive.
unsafe fn nsstring_owned(value: *const Object) -> Option<String> {
    if value.is_null() {
        return None;
    }
    let pointer: *const c_char = msg_send![value, UTF8String];
    let length: usize = msg_send![value, lengthOfBytesUsingEncoding: UTF8_ENCODING];
    if pointer.is_null() || length == 0 || length > MAX_LOCALE_BYTES {
        return None;
    }
    str::from_utf8(slice::from_raw_parts(pointer.cast(), length))
        .ok()
        .filter(|value| !value.contains('\0'))
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_locale_nil_is_unavailable() {
        assert!(unsafe { nsstring_owned(std::ptr::null()) }.is_none());
    }

    #[test]
    fn native_locale_is_owned_across_autorelease_pools() {
        let first = system_locale().expect("native Foundation locale must be readable");
        for _ in 0..64 {
            assert_eq!(system_locale().as_ref(), Some(&first));
        }
        assert!(first.ends_with(".UTF-8"));
        assert!(!first.contains('\0'));
    }
}
