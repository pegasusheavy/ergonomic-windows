//! CPU profiling binary for flamegraph analysis.
//!
//! Run with (requires Administrator on Windows):
//! ```bash
//! cargo flamegraph --bin cpu-profile
//! ```
//!
//! This generates flamegraph.svg showing CPU time spent in each function.

use ergonomic_windows::string::{from_wide, to_wide, WideString, WideStringBuilder, WideStringPool};
use std::hint::black_box;

fn main() {
    println!("=== CPU Profiling for ergonomic-windows ===\n");
    println!("Running CPU-intensive workloads for flamegraph analysis...\n");

    // Run each workload multiple times to get good sampling
    for iteration in 1..=3 {
        println!("--- Iteration {} ---", iteration);

        profile_string_conversions();
        profile_wide_string_sso();
        profile_wide_string_heap();
        profile_wide_string_pool();
        profile_wide_string_builder();
        profile_mixed_workload();
    }

    println!("\n=== CPU Profiling Complete ===");
    println!("If run with `cargo flamegraph`, check flamegraph.svg");
}

/// Profile basic string conversions - the most common operation
fn profile_string_conversions() {
    const ITERATIONS: usize = 100_000;

    // Small strings (most common case)
    for _ in 0..ITERATIONS {
        let wide = black_box(to_wide(black_box("Hello, World!")));
        let _ = black_box(from_wide(black_box(&wide)));
    }

    // Medium strings
    let medium = "a".repeat(100);
    for _ in 0..ITERATIONS / 10 {
        let wide = black_box(to_wide(black_box(&medium)));
        let _ = black_box(from_wide(black_box(&wide)));
    }

    // Large strings
    let large = "x".repeat(10_000);
    for _ in 0..ITERATIONS / 100 {
        let wide = black_box(to_wide(black_box(&large)));
        let _ = black_box(from_wide(black_box(&wide)));
    }

    println!("  String conversions: {} iterations", ITERATIONS);
}

/// Profile WideString with SSO (small string optimization)
fn profile_wide_string_sso() {
    const ITERATIONS: usize = 100_000;

    // Strings that fit in SSO buffer (≤22 chars)
    let small_strings = ["Hello", "World", "Test", "Rust", "Windows", "API"];

    for _ in 0..ITERATIONS {
        for s in &small_strings {
            let ws = black_box(WideString::new(black_box(*s)));
            let _ = black_box(ws.as_ptr());
            let _ = black_box(ws.len());
        }
    }

    println!("  WideString SSO: {} iterations", ITERATIONS * small_strings.len());
}

/// Profile WideString with heap allocation
fn profile_wide_string_heap() {
    const ITERATIONS: usize = 10_000;

    // Strings that exceed SSO buffer
    let long_string = "This is a longer string that definitely exceeds the SSO buffer size of 23";

    for _ in 0..ITERATIONS {
        let ws = black_box(WideString::new(black_box(long_string)));
        let _ = black_box(ws.as_ptr());
        let _ = black_box(ws.to_string_lossy());
    }

    println!("  WideString heap: {} iterations", ITERATIONS);
}

/// Profile WideStringPool for high-throughput scenarios
fn profile_wide_string_pool() {
    const ITERATIONS: usize = 50_000;

    let mut pool = WideStringPool::with_preallocated(16, 256);

    let strings = [
        "file1.txt",
        "file2.txt",
        "document.docx",
        "image.png",
        "config.json",
        "data.csv",
    ];

    for _ in 0..ITERATIONS {
        for s in &strings {
            let pooled = black_box(pool.get(black_box(*s)));
            let _ = black_box(pooled.as_ptr());
            pool.put(pooled);
        }
    }

    println!("  WideStringPool: {} iterations", ITERATIONS * strings.len());
}

/// Profile WideStringBuilder for incremental string building
fn profile_wide_string_builder() {
    const ITERATIONS: usize = 10_000;

    // Without preallocation
    for _ in 0..ITERATIONS {
        let mut builder = WideStringBuilder::new();
        for j in 0..10 {
            builder.push(black_box(&format!("segment_{}_", j)));
        }
        let _ = black_box(builder.build());
    }

    // With preallocation
    for _ in 0..ITERATIONS {
        let mut builder = WideStringBuilder::with_capacity(256);
        for j in 0..10 {
            builder.push(black_box(&format!("segment_{}_", j)));
        }
        let _ = black_box(builder.build());
    }

    println!("  WideStringBuilder: {} iterations", ITERATIONS * 2);
}

/// Profile a mixed workload simulating real-world usage
fn profile_mixed_workload() {
    const ITERATIONS: usize = 10_000;

    let paths = [
        "C:\\Windows\\System32\\kernel32.dll",
        "C:\\Program Files\\App\\config.json",
        "C:\\Users\\User\\Documents\\file.txt",
    ];

    let registry_keys = [
        "Software\\Microsoft\\Windows\\CurrentVersion",
        "SOFTWARE\\Classes\\.txt",
        "SYSTEM\\CurrentControlSet\\Services",
    ];

    for _ in 0..ITERATIONS {
        // Simulate path operations
        for path in &paths {
            let ws = black_box(WideString::new(black_box(*path)));
            let _ = black_box(ws.as_pcwstr());
        }

        // Simulate registry key operations
        for key in &registry_keys {
            let ws = black_box(WideString::new(black_box(*key)));
            let _ = black_box(ws.as_ptr());
            let _ = black_box(ws.len());
        }

        // Simulate command building
        let cmd = black_box(format!(
            "{} {} {}",
            black_box("cmd.exe"),
            black_box("/c"),
            black_box("echo hello")
        ));
        let _ = black_box(to_wide(black_box(&cmd)));
    }

    println!("  Mixed workload: {} iterations", ITERATIONS);
}

