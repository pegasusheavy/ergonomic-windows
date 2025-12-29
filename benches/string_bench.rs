//! Benchmarks for the string module.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ergonomic_windows::string::{from_wide, to_wide, WideString, WideStringBuilder};

fn bench_to_wide(c: &mut Criterion) {
    let mut group = c.benchmark_group("to_wide");

    for size in [10, 100, 1000, 10000].iter() {
        let input: String = "a".repeat(*size);
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &input, |b, input| {
            b.iter(|| to_wide(black_box(input)))
        });
    }

    group.finish();
}

fn bench_from_wide(c: &mut Criterion) {
    let mut group = c.benchmark_group("from_wide");

    for size in [10, 100, 1000, 10000].iter() {
        let input: String = "a".repeat(*size);
        let wide = to_wide(&input);
        group.throughput(Throughput::Bytes(*size as u64 * 2)); // UTF-16 is 2 bytes per char
        group.bench_with_input(BenchmarkId::from_parameter(size), &wide, |b, wide| {
            b.iter(|| from_wide(black_box(wide)))
        });
    }

    group.finish();
}

fn bench_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_roundtrip");

    for size in [10, 100, 1000, 10000].iter() {
        let input: String = "a".repeat(*size);
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &input, |b, input| {
            b.iter(|| {
                let wide = to_wide(black_box(input));
                from_wide(&wide)
            })
        });
    }

    group.finish();
}

fn bench_wide_string_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("WideString_creation");

    for size in [10, 100, 1000, 10000].iter() {
        let input: String = "a".repeat(*size);
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &input, |b, input| {
            b.iter(|| WideString::new(black_box(input)))
        });
    }

    group.finish();
}

fn bench_wide_string_builder(c: &mut Criterion) {
    let mut group = c.benchmark_group("WideStringBuilder");

    // Benchmark building strings incrementally
    group.bench_function("build_10_segments", |b| {
        b.iter(|| {
            let mut builder = WideStringBuilder::new();
            for _ in 0..10 {
                builder.push("Hello, World!");
            }
            builder.build()
        })
    });

    group.bench_function("build_100_segments", |b| {
        b.iter(|| {
            let mut builder = WideStringBuilder::new();
            for _ in 0..100 {
                builder.push("Hello, World!");
            }
            builder.build()
        })
    });

    // Compare with pre-allocated capacity
    group.bench_function("build_100_segments_preallocated", |b| {
        b.iter(|| {
            let mut builder = WideStringBuilder::with_capacity(100 * 13);
            for _ in 0..100 {
                builder.push("Hello, World!");
            }
            builder.build()
        })
    });

    group.finish();
}

fn bench_unicode_strings(c: &mut Criterion) {
    let mut group = c.benchmark_group("unicode_strings");

    // ASCII only
    let ascii = "Hello, World! This is a test string.";
    group.bench_function("ascii_to_wide", |b| b.iter(|| to_wide(black_box(ascii))));

    // Mixed Unicode with emojis
    let unicode = "Hello, World! \u{1F600}\u{1F601}\u{1F602} \u{4E2D}\u{6587}";
    group.bench_function("unicode_to_wide", |b| {
        b.iter(|| to_wide(black_box(unicode)))
    });

    // CJK characters (2 UTF-16 code units each for some)
    let cjk = "\u{4E2D}\u{6587}\u{65E5}\u{672C}\u{8A9E}".repeat(100);
    group.bench_function("cjk_to_wide", |b| b.iter(|| to_wide(black_box(&cjk))));

    group.finish();
}

fn bench_memory_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_patterns");

    // Test repeated allocation/deallocation pattern
    group.bench_function("repeated_alloc_dealloc", |b| {
        b.iter(|| {
            for _ in 0..100 {
                let s = to_wide("Test string for allocation");
                black_box(s);
            }
        })
    });

    // Test reuse pattern (single allocation, multiple uses)
    group.bench_function("single_alloc_reuse", |b| {
        let wide = to_wide("Test string for allocation");
        b.iter(|| {
            for _ in 0..100 {
                let _ = from_wide(black_box(&wide));
            }
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_to_wide,
    bench_from_wide,
    bench_roundtrip,
    bench_wide_string_creation,
    bench_wide_string_builder,
    bench_unicode_strings,
    bench_memory_patterns
);
criterion_main!(benches);
