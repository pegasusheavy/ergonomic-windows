//! Fuzz target for WideString creation and conversion.
//!
//! This tests that WideString can be created from any valid UTF-8 string
//! and converted back without loss (for strings without embedded nulls).
//!
//! Note: WideString stores the FULL content including embedded nulls,
//! but to_string_lossy() truncates at the first null (Windows convention).

#![no_main]

use libfuzzer_sys::fuzz_target;
use ergonomic_windows::string::WideString;

fuzz_target!(|data: &str| {
    // Create WideString from input - stores FULL content
    let wide = WideString::new(data);

    // WideString::len() returns the FULL UTF-16 length (not including terminating null)
    let full_utf16_len = data.encode_utf16().count();
    assert_eq!(wide.len(), full_utf16_len, "Length should match FULL UTF-16 code unit count");

    // Empty check is based on full content
    assert_eq!(wide.is_empty(), data.is_empty());

    // Pointer should not be null
    assert!(!wide.as_ptr().is_null(), "Pointer should not be null");

    // to_string_lossy() truncates at first embedded null (Windows convention)
    let back = wide.to_string_lossy();
    let expected = data.split('\0').next().unwrap_or("");
    assert_eq!(expected, back, "to_string_lossy should truncate at first null");
});

