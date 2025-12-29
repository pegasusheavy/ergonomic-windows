//! Memory profiling binary using dhat.
//!
//! Run with: cargo run --bin memory-profile --features dhat-heap --target x86_64-pc-windows-gnu

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

use ergonomic_windows::string::{from_wide, to_wide, WideString, WideStringBuilder};

fn main() {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    println!("=== Memory Profiling for ergonomic-windows ===\n");

    // Profile string conversions
    profile_string_conversions();

    // Profile WideString usage
    profile_wide_string();

    // Profile WideStringBuilder
    profile_wide_string_builder();

    // Profile repeated allocations
    profile_repeated_allocations();

    // Profile large strings
    profile_large_strings();

    println!("\n=== Profiling Complete ===");
    println!("Check dhat-heap.json for detailed heap analysis");
}

fn profile_string_conversions() {
    println!("--- String Conversions ---");

    // Small strings
    for _ in 0..1000 {
        let wide = to_wide("Hello, World!");
        let _ = from_wide(&wide);
    }

    // Medium strings
    let medium = "a".repeat(1000);
    for _ in 0..100 {
        let wide = to_wide(&medium);
        let _ = from_wide(&wide);
    }

    println!("Completed 1000 small + 100 medium string conversions");
}

fn profile_wide_string() {
    println!("--- WideString Creation ---");

    // Create many WideStrings
    let strings: Vec<WideString> = (0..1000)
        .map(|i| WideString::new(&format!("String number {}", i)))
        .collect();

    // Use them
    for s in &strings {
        let _ = s.as_ptr();
    }

    println!("Created and used 1000 WideStrings");
}

fn profile_wide_string_builder() {
    println!("--- WideStringBuilder ---");

    // Build strings incrementally
    for _ in 0..100 {
        let mut builder = WideStringBuilder::new();
        for j in 0..50 {
            builder.push(&format!("segment_{}_", j));
        }
        let _ = builder.build();
    }

    // With pre-allocation
    for _ in 0..100 {
        let mut builder = WideStringBuilder::with_capacity(1000);
        for j in 0..50 {
            builder.push(&format!("segment_{}_", j));
        }
        let _ = builder.build();
    }

    println!("Built 200 strings (100 without prealloc, 100 with prealloc)");
}

fn profile_repeated_allocations() {
    println!("--- Repeated Allocations Pattern ---");

    // Simulate a pattern that might cause fragmentation
    let mut results = Vec::new();

    for i in 0..500 {
        let s = to_wide(&format!("Iteration {}: Some data here", i));
        if i % 2 == 0 {
            results.push(s);
        }
        // Odd iterations drop immediately, even ones are kept
    }

    // Now drop half
    results.truncate(125);

    // Add more
    for i in 500..750 {
        results.push(to_wide(&format!("Second batch {}", i)));
    }

    println!("Completed fragmentation test with {} retained strings", results.len());
}

fn profile_large_strings() {
    println!("--- Large Strings ---");

    // Test with various large string sizes
    for size in [10_000, 100_000, 500_000] {
        let large = "x".repeat(size);
        let wide = to_wide(&large);
        let back = from_wide(&wide).unwrap();
        assert_eq!(large.len(), back.len());
        println!("Converted {} char string successfully", size);
    }
}
