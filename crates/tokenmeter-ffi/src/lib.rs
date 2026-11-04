#![expect(unsafe_code, reason = "the C ABI is raw pointers and exported symbols")]
#![allow(non_camel_case_types)]

use std::alloc::{Layout, alloc, dealloc};
use std::ffi::c_char;
use std::panic::catch_unwind;
use std::ptr;

use tokenmeter_core::Split;

pub const TOKENMETER_OK: u32 = 0;
pub const TOKENMETER_INVALID_UTF8: u32 = 1;
pub const TOKENMETER_INVALID_ARGUMENT: u32 = 2;
pub const TOKENMETER_INTERNAL: u32 = 3;
pub const TOKENMETER_NOT_UTF8: u32 = u32::MAX;

#[repr(C)]
#[derive(Debug)]
pub struct tokenmeter_groups {
    pub status: u32,
    pub tokens: u32,
    pub truncated: u32,
    pub groups: *mut u32,
    pub groups_len: usize,
}

/// # Safety
/// `p` is null with `len` zero, or points to `len` readable bytes.
unsafe fn text<'a>(p: *const u8, len: usize) -> Option<&'a str> {
    let bytes = match (p.is_null(), len) {
        (_, 0) => &[][..],
        (true, _) => return None,
        // SAFETY: the caller passes `len` readable bytes at `p`.
        (false, _) => unsafe { std::slice::from_raw_parts(p, len) },
    };

    std::str::from_utf8(bytes).ok()
}

/// # Safety
/// `input` is null with `input_len` zero, or points to `input_len` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tokenmeter_count(input: *const u8, input_len: usize) -> u32 {
    // SAFETY: forwarded from the caller.
    match unsafe { text(input, input_len) } {
        Some(text) => u32::try_from(tokenmeter_core::count(text)).unwrap_or(u32::MAX - 1),
        None => TOKENMETER_NOT_UTF8,
    }
}

/// # Safety
/// `input` is null with `input_len` zero, or points to `input_len` readable
/// bytes; `out` points to a writable `tokenmeter_groups`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tokenmeter_split(
    input: *const u8,
    input_len: usize,
    limit: usize,
    out: *mut tokenmeter_groups,
) {
    let mut result =
        tokenmeter_groups { status: TOKENMETER_OK, tokens: 0, truncated: 0, groups: ptr::null_mut(), groups_len: 0 };

    // SAFETY: forwarded from the caller.
    match unsafe { text(input, input_len) } {
        None => result.status = TOKENMETER_INVALID_UTF8,
        Some(_) if u32::try_from(input_len).is_err() => result.status = TOKENMETER_INVALID_ARGUMENT,
        Some(text) => match catch_unwind(|| tokenmeter_core::split(text, limit)) {
            Ok(split) => result = groups(&split),
            Err(_) => result.status = TOKENMETER_INTERNAL,
        },
    }

    if !out.is_null() {
        // SAFETY: the caller passes a writable `tokenmeter_groups`.
        unsafe { out.write(result) };
    }
}

fn groups(split: &Split) -> tokenmeter_groups {
    let pairs: Box<[u32]> = split.groups.iter().flat_map(|group| [group.end as u32, group.tokens as u32]).collect();

    tokenmeter_groups {
        status: TOKENMETER_OK,
        tokens: split.tokens as u32,
        truncated: u32::from(split.truncated),
        groups: Box::into_raw(pairs).cast::<u32>(),
        groups_len: split.groups.len(),
    }
}

/// # Safety
/// `split` is null or points to a `tokenmeter_groups` that `tokenmeter_split` wrote
/// and that has not been released.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tokenmeter_split_free(split: *mut tokenmeter_groups) {
    // SAFETY: the caller passes null or a valid `tokenmeter_groups`.
    let Some(split) = (unsafe { split.as_mut() }) else { return };

    if !split.groups.is_null() {
        let pairs = ptr::slice_from_raw_parts_mut(split.groups, split.groups_len * 2);
        // SAFETY: `groups` came from `Box::into_raw` of exactly this many pairs.
        drop(unsafe { Box::from_raw(pairs) });
    }

    split.groups = ptr::null_mut();
    split.groups_len = 0;
}

#[unsafe(no_mangle)]
pub extern "C" fn tokenmeter_prepare() {
    tokenmeter_core::prepare();
}

#[unsafe(no_mangle)]
pub extern "C" fn tokenmeter_encoding() -> *const c_char {
    c"o200k_base".as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn tokenmeter_version() -> *const c_char {
    const VERSION: &std::ffi::CStr =
        match std::ffi::CStr::from_bytes_with_nul(concat!(env!("CARGO_PKG_VERSION"), "\0").as_bytes()) {
            Ok(version) => version,
            Err(_) => panic!("the version has no NUL"),
        };

    VERSION.as_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn tokenmeter_alloc(len: usize) -> *mut u8 {
    match Layout::array::<u8>(len) {
        // SAFETY: the layout is valid and not zero-sized.
        Ok(layout) if len > 0 => unsafe { alloc(layout) },
        _ => ptr::null_mut(),
    }
}

/// # Safety
/// `p` is null or came from `tokenmeter_alloc(len)` and has not been released.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tokenmeter_dealloc(p: *mut u8, len: usize) {
    if let (false, Ok(layout)) = (p.is_null(), Layout::array::<u8>(len))
        && len > 0
    {
        // SAFETY: `p` came from `tokenmeter_alloc` with this layout.
        unsafe { dealloc(p, layout) };
    }
}

#[cfg(test)]
mod tests {
    use tokenmeter_core::Group;

    use super::*;

    #[test]
    #[cfg_attr(miri, ignore = "builds the whole o200k_base table")]
    fn splits_through_the_abi() {
        let text = "Пользователи — это наше всё.";
        let mut out = tokenmeter_groups { status: 9, tokens: 0, truncated: 0, groups: ptr::null_mut(), groups_len: 0 };

        // SAFETY: the text and `out` are valid for the call.
        unsafe { tokenmeter_split(text.as_ptr(), text.len(), 200, &raw mut out) };

        assert_eq!(out.status, TOKENMETER_OK);
        assert_eq!(out.tokens, 9);
        assert!(out.groups_len > 0);

        // SAFETY: `out` was written by `tokenmeter_split`.
        unsafe { tokenmeter_split_free(&raw mut out) };

        // SAFETY: the text is valid for the call.
        assert_eq!(unsafe { tokenmeter_count(text.as_ptr(), text.len()) }, 9);
    }

    #[test]
    fn hands_groups_over_and_takes_them_back() {
        let split = Split {
            tokens: 3,
            groups: vec![Group { end: 2, tokens: 1 }, Group { end: 5, tokens: 2 }],
            truncated: true,
        };
        let mut out = groups(&split);

        assert_eq!((out.status, out.tokens, out.truncated, out.groups_len), (TOKENMETER_OK, 3, 1, 2));
        // SAFETY: `groups` holds `groups_len` pairs.
        assert_eq!(unsafe { std::slice::from_raw_parts(out.groups, out.groups_len * 2) }, [2, 1, 5, 2]);

        // SAFETY: `out` came from `groups` and is released once; the second call sees null.
        unsafe {
            tokenmeter_split_free(&raw mut out);
            tokenmeter_split_free(&raw mut out);
            tokenmeter_split_free(ptr::null_mut());
        }

        assert!(out.groups.is_null());
        assert_eq!(out.groups_len, 0);

        let mut empty = groups(&Split { tokens: 0, groups: vec![], truncated: false });

        // SAFETY: `empty` came from `groups`.
        unsafe { tokenmeter_split_free(&raw mut empty) };
    }

    #[test]
    fn refuses_bad_input_before_tokenizing() {
        let mut out = tokenmeter_groups { status: 9, tokens: 0, truncated: 0, groups: ptr::null_mut(), groups_len: 0 };

        // SAFETY: the bytes and `out` are valid for the call.
        unsafe { tokenmeter_split([0xff].as_ptr(), 1, 200, &raw mut out) };
        assert_eq!(out.status, TOKENMETER_INVALID_UTF8);
        assert!(out.groups.is_null());

        // SAFETY: a null pointer with a length is refused before it is read.
        unsafe { tokenmeter_split(ptr::null(), 1, 200, &raw mut out) };
        assert_eq!(out.status, TOKENMETER_INVALID_UTF8);

        // SAFETY: a null `out` is skipped.
        unsafe { tokenmeter_split([0xff].as_ptr(), 1, 200, ptr::null_mut()) };

        // SAFETY: the bytes are valid for the call.
        assert_eq!(unsafe { tokenmeter_count([0xff].as_ptr(), 1) }, TOKENMETER_NOT_UTF8);
        // SAFETY: a null pointer with a length is refused before it is read.
        assert_eq!(unsafe { tokenmeter_count(ptr::null(), 1) }, TOKENMETER_NOT_UTF8);
    }

    #[test]
    fn allocates_and_releases_input() {
        let p = tokenmeter_alloc(16);

        assert!(!p.is_null());
        // SAFETY: `p` holds 16 writable bytes.
        unsafe { p.write_bytes(0x61, 16) };
        // SAFETY: `p` came from `tokenmeter_alloc(16)`.
        unsafe { tokenmeter_dealloc(p, 16) };

        assert!(tokenmeter_alloc(0).is_null());
        // SAFETY: null is ignored.
        unsafe { tokenmeter_dealloc(ptr::null_mut(), 0) };
    }
}
