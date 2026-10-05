// Copyright 2026 Automexia contributors. Licensed under MIT OR Apache-2.0.

pub(crate) const MAX_VALUE_UTF16: usize = 8192;

pub(crate) fn range_end(location: usize, length: usize) -> Option<usize> {
    location.checked_add(length)
}

pub(crate) fn value_allowed(
    declared: bool,
    read_only: bool,
    disabled: bool,
    utf16: usize,
) -> bool {
    declared && !read_only && !disabled && utf16 <= MAX_VALUE_UTF16
}

pub(crate) fn decode_value(units: &[u16]) -> Option<String> {
    if units.len() > MAX_VALUE_UTF16 {
        return None;
    }
    String::from_utf16(units).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foreign_range_overflow_and_maximum_empty_range() {
        assert_eq!(range_end(1, usize::MAX), None);
        assert_eq!(range_end(usize::MAX, 1), None);
        assert_eq!(range_end(usize::MAX, 0), Some(usize::MAX));
        assert_eq!(range_end(2, 3), Some(5));
    }

    #[test]
    fn value_requests_require_declared_writable_enabled_bounded_input() {
        assert!(value_allowed(true, false, false, MAX_VALUE_UTF16));
        assert!(!value_allowed(true, false, false, MAX_VALUE_UTF16 + 1));
        assert!(!value_allowed(true, false, false, usize::MAX));
        assert!(!value_allowed(false, false, false, 0));
        assert!(!value_allowed(true, true, false, 0));
        assert!(!value_allowed(true, false, true, 0));
    }

    #[test]
    fn malformed_native_strings_are_rejected_without_panicking() {
        assert_eq!(decode_value(&[0xd800]), None);
        assert_eq!(decode_value(&[0xdc00]), None);
        assert_eq!(decode_value(&[0xd83d, 0xdc69]), Some("\u{1f469}".into()));
        assert_eq!(decode_value(&vec![0x41; MAX_VALUE_UTF16 + 1]), None);
    }
}
