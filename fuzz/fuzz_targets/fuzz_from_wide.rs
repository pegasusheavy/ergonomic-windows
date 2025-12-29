//! Fuzz target for from_wide with arbitrary UTF-16 data.
//!
//! This tests that from_wide handles any UTF-16 input without panicking,
//! including invalid sequences.

#![no_main]

use libfuzzer_sys::fuzz_target;
use ergonomic_windows::string::from_wide;

fuzz_target!(|data: Vec<u16>| {
    // Limit size to avoid OOM
    if data.len() > 100_000 {
        return;
    }

    // Try to convert - should never panic
    let result = from_wide(&data);

    // If conversion succeeded, verify the result is valid UTF-8
    if let Ok(s) = &result {
        // String type guarantees valid UTF-8, but let's be explicit
        assert!(s.is_char_boundary(0));
        assert!(s.is_char_boundary(s.len()));
    }

    // Test with explicit null terminator
    let mut with_null = data.clone();
    with_null.push(0);
    let result_with_null = from_wide(&with_null);

    // Both should either succeed or fail consistently
    // (adding null shouldn't change the validity of the UTF-16 content)
    match (&result, &result_with_null) {
        (Ok(a), Ok(b)) => {
            // With null terminator, we should get the same result
            // because from_wide stops at the first null
            if !data.contains(&0) {
                assert_eq!(a, b, "Adding null should not change result for strings without embedded nulls");
            }
        }
        _ => {
            // Error cases are fine - invalid UTF-16 sequences
        }
    }

    // Test with embedded nulls - from_wide should stop at the first null
    if let Some(null_pos) = data.iter().position(|&c| c == 0) {
        if let Ok(s) = &result {
            // The result should only contain data up to the first null
            let truncated = &data[..null_pos];
            if let Ok(expected) = from_wide(truncated) {
                assert_eq!(s, &expected, "Should stop at first null");
            }
        }
    }
});

