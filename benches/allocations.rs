//! Allocation tracking benchmarks to identify memory usage patterns.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ergonomic_windows::string::{from_wide, to_wide, WideString, WideStringBuilder};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

/// A tracking allocator that counts allocations and total bytes allocated.
struct TrackingAllocator {
    allocation_count: AtomicUsize,
    deallocation_count: AtomicUsize,
    bytes_allocated: AtomicUsize,
    bytes_deallocated: AtomicUsize,
    peak_memory: AtomicUsize,
    current_memory: AtomicUsize,
}

impl TrackingAllocator {
    const fn new() -> Self {
        Self {
            allocation_count: AtomicUsize::new(0),
            deallocation_count: AtomicUsize::new(0),
            bytes_allocated: AtomicUsize::new(0),
            bytes_deallocated: AtomicUsize::new(0),
            peak_memory: AtomicUsize::new(0),
            current_memory: AtomicUsize::new(0),
        }
    }

    fn reset(&self) {
        self.allocation_count.store(0, Ordering::SeqCst);
        self.deallocation_count.store(0, Ordering::SeqCst);
        self.bytes_allocated.store(0, Ordering::SeqCst);
        self.bytes_deallocated.store(0, Ordering::SeqCst);
        self.peak_memory.store(0, Ordering::SeqCst);
        self.current_memory.store(0, Ordering::SeqCst);
    }

    fn stats(&self) -> AllocationStats {
        AllocationStats {
            allocations: self.allocation_count.load(Ordering::SeqCst),
            deallocations: self.deallocation_count.load(Ordering::SeqCst),
            bytes_allocated: self.bytes_allocated.load(Ordering::SeqCst),
            bytes_deallocated: self.bytes_deallocated.load(Ordering::SeqCst),
            peak_memory: self.peak_memory.load(Ordering::SeqCst),
        }
    }
}

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() {
            self.allocation_count.fetch_add(1, Ordering::SeqCst);
            self.bytes_allocated
                .fetch_add(layout.size(), Ordering::SeqCst);
            let current = self
                .current_memory
                .fetch_add(layout.size(), Ordering::SeqCst)
                + layout.size();

            // Update peak memory
            let mut peak = self.peak_memory.load(Ordering::SeqCst);
            while current > peak {
                match self.peak_memory.compare_exchange_weak(
                    peak,
                    current,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                ) {
                    Ok(_) => break,
                    Err(p) => peak = p,
                }
            }
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.deallocation_count.fetch_add(1, Ordering::SeqCst);
        self.bytes_deallocated
            .fetch_add(layout.size(), Ordering::SeqCst);
        self.current_memory
            .fetch_sub(layout.size(), Ordering::SeqCst);
        System.dealloc(ptr, layout)
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct AllocationStats {
    allocations: usize,
    deallocations: usize,
    bytes_allocated: usize,
    bytes_deallocated: usize,
    peak_memory: usize,
}

impl AllocationStats {
    fn leaked_bytes(&self) -> usize {
        self.bytes_allocated.saturating_sub(self.bytes_deallocated)
    }

    #[allow(dead_code)]
    fn leaked_allocations(&self) -> usize {
        self.allocations.saturating_sub(self.deallocations)
    }
}

#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator::new();

fn bench_allocation_counts(c: &mut Criterion) {
    let mut group = c.benchmark_group("allocation_analysis");

    // Measure allocations for to_wide
    group.bench_function("to_wide_small_allocations", |b| {
        b.iter_custom(|iters| {
            ALLOCATOR.reset();
            let start = std::time::Instant::now();
            for _ in 0..iters {
                let _ = to_wide(black_box("Hello, World!"));
            }
            let elapsed = start.elapsed();
            let stats = ALLOCATOR.stats();

            // Print allocation info on first run
            if iters == 1 {
                eprintln!(
                    "\nto_wide small: {} allocs, {} bytes per call",
                    stats.allocations, stats.bytes_allocated
                );
            }
            elapsed
        });
    });

    // Measure allocations for from_wide
    group.bench_function("from_wide_small_allocations", |b| {
        let wide = to_wide("Hello, World!");
        b.iter_custom(|iters| {
            ALLOCATOR.reset();
            let start = std::time::Instant::now();
            for _ in 0..iters {
                let _ = from_wide(black_box(&wide));
            }
            let elapsed = start.elapsed();
            let stats = ALLOCATOR.stats();

            if iters == 1 {
                eprintln!(
                    "\nfrom_wide small: {} allocs, {} bytes per call",
                    stats.allocations, stats.bytes_allocated
                );
            }
            elapsed
        });
    });

    // Measure WideString allocations
    group.bench_function("WideString_allocations", |b| {
        b.iter_custom(|iters| {
            ALLOCATOR.reset();
            let start = std::time::Instant::now();
            for _ in 0..iters {
                let _ = WideString::new(black_box("Hello, World!"));
            }
            let elapsed = start.elapsed();
            let stats = ALLOCATOR.stats();

            if iters == 1 {
                eprintln!(
                    "\nWideString: {} allocs, {} bytes per call",
                    stats.allocations, stats.bytes_allocated
                );
            }
            elapsed
        });
    });

    // Measure WideStringBuilder allocations
    group.bench_function("WideStringBuilder_allocations", |b| {
        b.iter_custom(|iters| {
            ALLOCATOR.reset();
            let start = std::time::Instant::now();
            for _ in 0..iters {
                let mut builder = WideStringBuilder::new();
                builder.push("Hello, ");
                builder.push("World!");
                let _ = builder.build();
            }
            let elapsed = start.elapsed();
            let stats = ALLOCATOR.stats();

            if iters == 1 {
                eprintln!(
                    "\nWideStringBuilder: {} allocs, {} bytes per call, {} leaked",
                    stats.allocations,
                    stats.bytes_allocated,
                    stats.leaked_bytes()
                );
            }
            elapsed
        });
    });

    group.finish();
}

fn bench_memory_leaks(c: &mut Criterion) {
    let mut group = c.benchmark_group("leak_detection");

    // Test for leaks in repeated operations
    group.bench_function("string_roundtrip_leak_check", |b| {
        b.iter_custom(|iters| {
            ALLOCATOR.reset();
            let start = std::time::Instant::now();
            for _ in 0..iters {
                let wide = to_wide(black_box("Test for leaks"));
                let _ = from_wide(&wide);
            }
            let elapsed = start.elapsed();
            let stats = ALLOCATOR.stats();

            if stats.leaked_bytes() > 0 && iters > 0 {
                eprintln!(
                    "\nWARNING: Potential leak detected! {} bytes leaked after {} iterations",
                    stats.leaked_bytes(),
                    iters
                );
            }
            elapsed
        });
    });

    // Test WideStringBuilder reuse pattern
    group.bench_function("builder_reuse_pattern", |b| {
        b.iter_custom(|iters| {
            ALLOCATOR.reset();
            let start = std::time::Instant::now();
            for _ in 0..iters {
                let mut builder = WideStringBuilder::with_capacity(256);
                for j in 0..10 {
                    builder.push(&format!("segment_{}", j));
                }
                let result = builder.build();
                black_box(result);
            }
            let elapsed = start.elapsed();
            let stats = ALLOCATOR.stats();

            if iters == 1 {
                eprintln!(
                    "\nBuilder reuse: {} allocs, {} peak bytes, {} leaked",
                    stats.allocations,
                    stats.peak_memory,
                    stats.leaked_bytes()
                );
            }
            elapsed
        });
    });

    group.finish();
}

fn bench_large_allocations(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_allocations");

    // Test with progressively larger strings
    for size in [1_000, 10_000, 100_000, 1_000_000].iter() {
        let input: String = "x".repeat(*size);

        group.bench_function(format!("to_wide_{}_chars", size), |b| {
            b.iter_custom(|iters| {
                ALLOCATOR.reset();
                let start = std::time::Instant::now();
                for _ in 0..iters {
                    let _ = to_wide(black_box(&input));
                }
                let elapsed = start.elapsed();
                let stats = ALLOCATOR.stats();

                if iters == 1 {
                    eprintln!(
                        "\nto_wide {} chars: {} allocs, {} bytes, {} bytes/char",
                        size,
                        stats.allocations,
                        stats.bytes_allocated,
                        stats.bytes_allocated / *size
                    );
                }
                elapsed
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_allocation_counts,
    bench_memory_leaks,
    bench_large_allocations
);
criterion_main!(benches);
