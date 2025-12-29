//! Fuzz target for WideStringBuilder.
//!
//! This tests the incremental string building functionality.

#![no_main]

use libfuzzer_sys::fuzz_target;
use arbitrary::Arbitrary;
use ergonomic_windows::string::{WideStringBuilder, from_wide};

#[derive(Debug, Arbitrary)]
struct BuilderInput {
    segments: Vec<String>,
    with_capacity: Option<usize>,
}

fuzz_target!(|input: BuilderInput| {
    // Limit segment count and sizes to avoid OOM
    if input.segments.len() > 100 {
        return;
    }

    let total_len: usize = input.segments.iter().map(|s| s.len()).sum();
    if total_len > 1_000_000 {
        return;
    }

    // Create builder (with or without capacity)
    let mut builder = match input.with_capacity {
        Some(cap) if cap < 10000 => WideStringBuilder::with_capacity(cap),
        _ => WideStringBuilder::new(),
    };

    // Build the expected result
    let mut expected = String::new();
    for segment in &input.segments {
        builder.push(segment);
        expected.push_str(segment);
    }

    // Check length before building
    let expected_utf16_len = expected.encode_utf16().count();
    assert_eq!(builder.len(), expected_utf16_len, "Builder length should match");
    assert_eq!(builder.is_empty(), expected.is_empty());

    // Build the final string
    let wide = builder.build();

    // Verify null termination
    assert!(wide.last() == Some(&0), "Must be null-terminated");

    // Convert back and verify
    if let Ok(back) = from_wide(&wide) {
        assert_eq!(expected, back, "Built string should match expected");
    }
});

