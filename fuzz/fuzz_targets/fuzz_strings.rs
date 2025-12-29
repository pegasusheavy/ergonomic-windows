//! Fuzz target for string conversion roundtrip.
//!
//! This tests that `to_wide` -> `from_wide` produces the original string
//! for any valid UTF-8 input.

#![no_main]

use libfuzzer_sys::fuzz_target;
use ergonomic_windows::string::{to_wide, from_wide};

fuzz_target!(|data: &str| {
    // Convert to wide string (UTF-16)
    let wide = to_wide(data);

    // Verify the wide string is null-terminated
    assert!(wide.last() == Some(&0), "Wide string must be null-terminated");

    // Convert back to UTF-8
    if let Ok(back) = from_wide(&wide) {
        // Should match the original
        assert_eq!(data, back, "Roundtrip should preserve string content");
    }

    // Also test without the null terminator
    if wide.len() > 1 {
        let without_null = &wide[..wide.len() - 1];
        if let Ok(back) = from_wide(without_null) {
            assert_eq!(data, back, "Roundtrip without null should also work");
        }
    }
});

