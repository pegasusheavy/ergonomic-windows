//! Fuzz target for WideString creation and conversion.
//!
//! This tests that WideString can be created from any valid UTF-8 string
//! and converted back without loss.

#![no_main]

use libfuzzer_sys::fuzz_target;
use ergonomic_windows::string::WideString;

fuzz_target!(|data: &str| {
    // Create WideString from input
    let wide = WideString::new(data);

    // Check length is correct (should be string length, not including null)
    // Note: UTF-16 can have different length than UTF-8 for non-BMP characters
    let expected_len = data.encode_utf16().count();
    assert_eq!(wide.len(), expected_len, "Length should match UTF-16 code unit count");

    // Empty string should report as empty
    assert_eq!(wide.is_empty(), data.is_empty());

    // Pointer should not be null
    assert!(!wide.as_ptr().is_null(), "Pointer should not be null");

    // Convert back to String
    let back = wide.to_string_lossy();

    // For valid UTF-8 input, roundtrip should work
    // (to_string_lossy may replace invalid sequences, but our input is valid UTF-8)
    assert_eq!(data, back, "Roundtrip should preserve content");
});

