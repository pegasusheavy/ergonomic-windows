//! Fuzz target for WideStringBuilder.
//!
//! This tests the incremental string building functionality.
//!
//! Note: Strings with embedded nulls (U+0000) will be truncated when
//! converting back via from_wide, as Windows uses null-terminated strings.

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

    // Build the full result (including any embedded nulls)
    let mut full_expected = String::new();
    for segment in &input.segments {
        builder.push(segment);
        full_expected.push_str(segment);
    }

    // The effective result after from_wide (truncated at first null)
    let effective_expected = full_expected.split('\0').next().unwrap_or("");

    // Check length before building - builder tracks full length including nulls
    let full_utf16_len = full_expected.encode_utf16().count();
    assert_eq!(builder.len(), full_utf16_len, "Builder length should match full content");
    assert_eq!(builder.is_empty(), full_expected.is_empty());

    // Build the final string
    let wide = builder.build();

    // Verify null termination
    assert!(wide.last() == Some(&0), "Must be null-terminated");

    // Convert back and verify - will truncate at first embedded null
    if let Ok(back) = from_wide(&wide) {
        assert_eq!(effective_expected, back, "Built string should match expected (up to first null)");
    }
});

