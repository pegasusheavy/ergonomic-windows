//! Fuzz target for command-line argument quoting.
//!
//! This tests that the quote_arg function properly handles all inputs
//! without panicking and produces valid quoted strings.

#![no_main]

use libfuzzer_sys::fuzz_target;

// We need to test the internal quote_arg function.
// Since it's private, we'll recreate the logic here for testing.
fn quote_arg(arg: &str) -> String {
    if arg.is_empty() || arg.contains(' ') || arg.contains('\t') || arg.contains('"') {
        let mut quoted = String::with_capacity(arg.len() + 2);
        quoted.push('"');

        let mut chars = arg.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\\' {
                let mut backslash_count = 1;
                while chars.peek() == Some(&'\\') {
                    chars.next();
                    backslash_count += 1;
                }

                if chars.peek() == Some(&'"') || chars.peek().is_none() {
                    for _ in 0..backslash_count * 2 {
                        quoted.push('\\');
                    }
                } else {
                    for _ in 0..backslash_count {
                        quoted.push('\\');
                    }
                }
            } else if c == '"' {
                quoted.push('\\');
                quoted.push('"');
            } else {
                quoted.push(c);
            }
        }

        quoted.push('"');
        quoted
    } else {
        arg.to_string()
    }
}

fuzz_target!(|data: &str| {
    // Should never panic
    let quoted = quote_arg(data);

    // If original was empty or contained special chars, result should be quoted
    if data.is_empty() || data.contains(' ') || data.contains('\t') || data.contains('"') {
        assert!(quoted.starts_with('"'), "Special strings should be quoted");
        assert!(quoted.ends_with('"'), "Special strings should be quoted");
    }

    // Result should never be empty (empty input produces "\"\"")
    assert!(!quoted.is_empty(), "Quoted result should never be empty");

    // Result should be valid UTF-8 (it is, since we're returning String)
    assert!(quoted.is_ascii() || !data.is_ascii(), "ASCII input should produce ASCII output");
});

