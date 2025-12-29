# TODO: Performance and Memory Improvements

This document outlines findings from static code analysis, benchmarks, and memory profiling for the `ergonomic-windows` crate.

## Running Benchmarks

```bash
# Run all benchmarks (native Windows)
cargo bench

# Run specific benchmark
cargo bench --bench string_bench

# Run allocation tracking benchmarks
cargo bench --bench allocations

# Run memory profiler (generates dhat-heap.json)
cargo run --bin memory-profile --features dhat-heap
```

## Running Fuzz Tests

```bash
# Install cargo-fuzz (requires nightly)
cargo install cargo-fuzz

# Run a fuzz target
cd fuzz
cargo +nightly fuzz run fuzz_strings

# Run with time limit (30 seconds)
cargo +nightly fuzz run fuzz_strings -- -max_total_time=30

# List all fuzz targets
cargo +nightly fuzz list
```

### Available Fuzz Targets

| Target | Description |
|--------|-------------|
| `fuzz_strings` | Tests `to_wide` → `from_wide` roundtrip |
| `fuzz_quote_arg` | Tests command-line argument quoting |
| `fuzz_wide_string` | Tests `WideString` creation and conversion |
| `fuzz_wide_builder` | Tests `WideStringBuilder` incremental building |
| `fuzz_from_wide` | Tests `from_wide` with arbitrary UTF-16 data |

---

## Benchmark Results (December 2025)

### String Conversion Performance (After Optimization)

| Benchmark | Time | Improvement |
|-----------|------|-------------|
| `to_wide` (small) | ~55 ns | **-65% from 160ns** |
| `from_wide` (small) | ~64 ns | **-22%** |
| `WideString::new` | ~55 ns | **-62%** |
| `to_wide_1000_chars` | 616 ns | -12% |
| `to_wide_10000_chars` | 5.9 µs | **-30%** |
| `string_roundtrip` | 106 ns | **-58%** |

### WideString Performance

| Benchmark | Time | Notes |
|-----------|------|-------|
| `WideString_creation/10` | 101.26 ns | 94.18 MiB/s |
| `WideString_creation/100` | 197.84 ns | 482.05 MiB/s |
| `WideString_creation/1000` | 722.03 ns | 1.29 GiB/s |
| `WideString_creation/10000` | 5.93 µs | 1.57 GiB/s |

### WideStringBuilder Performance

| Benchmark | Time | Notes |
|-----------|------|-------|
| `build_10_segments` | 332.65 ns | - |
| `build_100_segments` | 1.43 µs | No preallocation |
| `build_100_segments_preallocated` | 1.04 µs | **27% faster with prealloc** |

### Unicode Handling

| Benchmark | Time |
|-----------|------|
| `ascii_to_wide` | 110.96 ns |
| `unicode_to_wide` | 113.10 ns |
| `cjk_to_wide (500 chars)` | 593.39 ns |

---

## Allocation Analysis Results (After Optimization)

### Per-Operation Allocation Counts

| Operation | Allocations | Bytes | Notes |
|-----------|-------------|-------|-------|
| `to_wide("Hello, World!")` | **1** | 28 | ✅ **Fixed!** |
| `from_wide` (13 chars) | 1 | 13 | ✅ Optimal |
| `WideString::new` | **1** | 28 | ✅ **Fixed!** |
| `WideStringBuilder` (2 pushes) | 3 | 56 | Vec growth (expected) |

### Large String Allocations

| Size | Allocations | Bytes | Bytes/Char |
|------|-------------|-------|------------|
| 1,000 chars | **1** | 2,002 | **2.0** ✅ |
| 10,000 chars | **1** | 20,002 | **2.0** ✅ |
| 100,000 chars | **1** | 200,002 | **2.0** ✅ |
| 1,000,000 chars | **1** | 2,000,002 | **2.0** ✅ |

### Memory Profiling Summary (dhat)

```
Total:     4,958,688 bytes in 22,426 blocks
At t-gmax: 2,001,040 bytes in 4 blocks
At t-end:  1,024 bytes in 1 blocks
```

---

## Completed Optimizations ✅

### 1. ✅ Fixed: Excessive Allocations in `to_wide` (string.rs)

**Status:** COMPLETED - Reduced from 3 allocations to 1 allocation
**Performance Gain:** -65% time reduction

```rust
// Now uses pre-allocated buffer with str::encode_utf16()
#[inline]
pub fn to_wide(s: &str) -> Vec<u16> {
    let mut result = Vec::with_capacity(s.len() + 1);
    result.extend(s.encode_utf16());
    result.push(0);
    result
}
```

---

### 2. ✅ Fixed: Unnecessary Allocations in `quote_arg` (process.rs)

**Status:** COMPLETED - Now uses `Cow<str>` to avoid allocation when no quoting needed

```rust
fn quote_arg(arg: &str) -> Cow<'_, str> {
    if needs_quoting { Cow::Owned(quoted) } else { Cow::Borrowed(arg) }
}
```

---

### 3. ✅ Fixed: Double Allocation in `from_wide` (string.rs)

**Status:** COMPLETED - Now uses `String::from_utf16` directly
**Performance Gain:** -22% time reduction

```rust
pub fn from_wide(wide: &[u16]) -> Result<String> {
    let len = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
    String::from_utf16(&wide[..len])
        .map_err(|_| Error::string_conversion("Invalid UTF-16 sequence"))
}
```

---

### 4. ✅ Fixed: Thread Handle (process.rs)

**Status:** COMPLETED - Now closes handle directly with `CloseHandle`

---

## Completed Performance Improvements ✅

### 5. ✅ Added `#[inline]` Hints to Hot Paths

**Status:** COMPLETED - Added to `to_wide`, `from_wide`, `as_raw`, `as_ptr`, `as_pcwstr`, and other hot functions.

---

### 6. ✅ Pre-calculate Command Line Length (process.rs)

**Status:** COMPLETED - `build_command_line` now pre-allocates with estimated capacity.

---

### 7. ✅ Add `WideStringBuilder::clear()` for Reuse

**Status:** COMPLETED - Added `clear()` and `build_and_clear()` methods.

---

### 8. ✅ Registry Buffer Optimization

**Status:** COMPLETED - Added `shrink_to_fit()` after truncate.

---

### 9. ✅ Add `Clone` for WideString

**Status:** COMPLETED - Added `#[derive(Clone)]` to `WideString`.

---

### 10. ✅ Add `WideString::with_capacity`

**Status:** COMPLETED - Added `with_capacity()` method.

---

## ✅ Completed Performance Improvements

### 11. ✅ Small String Optimization for `WideString`

**Status:** COMPLETED
**Impact:** Strings ≤22 UTF-16 chars stored inline with **ZERO heap allocation**
**Benchmark:** -78% time, 100% allocation reduction for short strings

```rust
const INLINE_CAP: usize = 23;  // Fits in 48 bytes with length

pub struct WideString {
    repr: WideStringRepr,
}

enum WideStringRepr {
    Inline { buf: [u16; INLINE_CAP], len: u8 },
    Heap(Vec<u16>),
}
```

**New Methods:**
- `is_inline()` - Check if string is stored inline
- `from_vec()` - Create from existing Vec<u16>

---

### 12. ✅ Object Pool for High-Throughput Scenarios

**Status:** COMPLETED
**Impact:** Reusable buffers for tight loops

```rust
let mut pool = WideStringPool::new();

// Get a pooled string (may reuse buffer)
let wide = pool.get("Hello, World!");

// Use with Windows APIs
some_api(wide.as_pcwstr());

// Return to pool for reuse
pool.put(wide);
```

**Pool Methods:**
- `new()` / `with_limits()` / `with_preallocated()` - Create pools
- `get()` / `get_path()` - Get strings from pool
- `put()` - Return strings to pool
- `len()` / `is_empty()` / `clear()` / `shrink_to()`

**PooledWideString Methods:**
- `as_ptr()` / `as_pcwstr()` - For Windows APIs
- `into_vec()` / `into_wide_string()` - Consume and convert

---

## Memory Leak Checklist

- [x] `OwnedHandle` - Properly closes handles in `Drop`
- [x] `WideString` - Uses `Vec` which is properly dropped
- [x] `WideStringBuilder` - Consumes self in `build()`, no leak
- [x] `Process` - Uses `OwnedHandle` for handle management
- [x] `Key` (registry) - Properly closes key in `Drop`
- [x] `Window` - Properly destroys window and unregisters class in `Drop`
- [ ] ⚠️ Minor: 1,024 bytes retained at program end in profiling (investigate)

---

## Priority Order

### ✅ Completed (December 2025)
- [x] **#1 - Fix `to_wide` allocations** - 3x allocation reduction, -65% time
- [x] #2 - `Cow<str>` in `quote_arg` - Avoid allocation when no quoting needed
- [x] #3 - Direct UTF-16 conversion in `from_wide` - -22% time
- [x] #4 - Direct handle close
- [x] #5 - Add `#[inline]` hints to hot paths
- [x] #6 - Pre-calculate command line length
- [x] #7 - Add `WideStringBuilder::clear()` and `build_and_clear()`
- [x] #8 - Registry buffer optimization with `shrink_to_fit()`
- [x] #9 - Add Clone for WideString
- [x] #10 - Add `WideString::with_capacity`

### ✅ Completed (Late December 2025)
- [x] #11 - Small string optimization for WideString
- [x] #12 - Object pooling (WideStringPool)

---

## Test Coverage Improvements

- [x] **Fuzzing** - Added 5 fuzz targets for string operations
- [ ] Add tests for unicode edge cases (surrogate pairs, BOM)
- [ ] Add tests for MAX_PATH length paths
- [ ] Add tests for empty registry values
- [ ] Add integration tests for process spawning
- [ ] Add stress tests for handle management
- [ ] Run fuzz tests and fix any discovered issues

---

## Notes

- All benchmarks run on native Windows (x86_64-pc-windows-msvc)
- Memory profiling with dhat requires the `dhat-heap` feature
- Allocation tracking uses a custom global allocator in benchmarks
- Consider using `cargo flamegraph` for detailed CPU profiling
- ✅ Vec growth pattern in `to_wide` has been fixed (now single allocation)
