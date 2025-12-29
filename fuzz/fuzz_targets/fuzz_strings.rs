//! Fuzz target for string conversion roundtrip.
//!
//! This tests that `to_wide` -> `from_wide` produces the original string
//! for any valid UTF-8 input that doesn't contain embedded null characters.
//!
//! Note: Strings with embedded nulls (U+0000) will be truncated by `from_wide`
//! because Windows APIs use null-terminated strings. This is expected behavior.

#![no_main]

use libfuzzer_sys::fuzz_target;
use ergonomic_windows::string::{to_wide, from_wide};

fuzz_target!(|data: &str| {
    // Convert to wide string (UTF-16)
    let wide = to_wide(data);

    // Verify the wide string is null-terminated
    assert!(wide.last() == Some(&0), "Wide string must be null-terminated");

    // Verify wide string has correct length (each UTF-16 code unit + null terminator)
    // Note: surrogate pairs use 2 code units
    let expected_len: usize = data.encode_utf16().count() + 1;
    assert_eq!(wide.len(), expected_len, "Wide string length mismatch");

    // Convert back to UTF-8
    if let Ok(back) = from_wide(&wide) {
        // For strings WITHOUT embedded nulls, roundtrip should be exact
        if !data.contains('\0') {
            assert_eq!(data, back, "Roundtrip should preserve string content");
        } else {
            // For strings WITH embedded nulls, from_wide truncates at first null
            // This is expected Windows behavior for null-terminated strings
            let expected = data.split('\0').next().unwrap_or("");
            assert_eq!(expected, back, "Should truncate at embedded null");
        }
    }

    // Test without the null terminator (should produce same result)
    if wide.len() > 1 {
        let without_null = &wide[..wide.len() - 1];
        if let Ok(back) = from_wide(without_null) {
            if !data.contains('\0') {
                assert_eq!(data, back, "Roundtrip without terminator should also work");
            }
        }
    }
});

