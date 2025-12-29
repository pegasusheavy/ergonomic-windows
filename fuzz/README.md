# Fuzzing for ergonomic-windows

This directory contains fuzz targets for the `ergonomic-windows` crate using [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz).

## Prerequisites

1. **Install cargo-fuzz** (requires nightly Rust):
   ```bash
   cargo install cargo-fuzz
   ```

2. **Switch to nightly Rust** (cargo-fuzz requires nightly):
   ```bash
   rustup default nightly
   # or use: cargo +nightly fuzz ...
   ```

## Available Fuzz Targets

| Target | Description |
|--------|-------------|
| `fuzz_strings` | Tests `to_wide` → `from_wide` roundtrip conversion |
| `fuzz_quote_arg` | Tests command-line argument quoting |
| `fuzz_wide_string` | Tests `WideString` creation and conversion |
| `fuzz_wide_builder` | Tests `WideStringBuilder` incremental building |
| `fuzz_from_wide` | Tests `from_wide` with arbitrary UTF-16 data |

## Running Fuzzers

### Run a specific fuzz target

```bash
cd fuzz
cargo +nightly fuzz run fuzz_strings
```

### Run with a time limit

```bash
cargo +nightly fuzz run fuzz_strings -- -max_total_time=60
```

### Run with more parallelism

```bash
cargo +nightly fuzz run fuzz_strings -- -jobs=4 -workers=4
```

### List all fuzz targets

```bash
cargo +nightly fuzz list
```

### View coverage

```bash
cargo +nightly fuzz coverage fuzz_strings
```

## Corpus Management

### Initial corpus

Create an initial corpus with interesting test cases:

```bash
mkdir -p corpus/fuzz_strings
echo -n "Hello, World!" > corpus/fuzz_strings/hello
echo -n "" > corpus/fuzz_strings/empty
echo -n "🎉🌍🚀" > corpus/fuzz_strings/emoji
```

### Run with corpus

```bash
cargo +nightly fuzz run fuzz_strings corpus/fuzz_strings
```

## Reproducing Crashes

If a crash is found, it will be saved to `artifacts/fuzz_target_name/`. To reproduce:

```bash
cargo +nightly fuzz run fuzz_strings artifacts/fuzz_strings/crash-*
```

## What We're Testing

### String Conversion (`fuzz_strings`, `fuzz_wide_string`)
- UTF-8 to UTF-16 conversion correctness
- Null termination
- Handling of all Unicode code points
- Surrogate pair handling

### UTF-16 Parsing (`fuzz_from_wide`)
- Invalid UTF-16 sequences (unpaired surrogates)
- Embedded null characters
- Very long strings
- Empty input

### Argument Quoting (`fuzz_quote_arg`)
- Special characters (spaces, tabs, quotes)
- Backslash escaping
- Empty strings
- Long strings

### Builder Pattern (`fuzz_wide_builder`)
- Multiple segment concatenation
- Capacity pre-allocation
- Empty segments
- Very long builds

## Adding New Fuzz Targets

1. Create a new file in `fuzz_targets/`:
   ```rust
   #![no_main]
   use libfuzzer_sys::fuzz_target;

   fuzz_target!(|data: &[u8]| {
       // Your fuzzing logic here
   });
   ```

2. Add entry to `Cargo.toml`:
   ```toml
   [[bin]]
   name = "fuzz_new_target"
   path = "fuzz_targets/fuzz_new_target.rs"
   test = false
   doc = false
   bench = false
   ```

## CI Integration

For CI, run fuzzers for a limited time to catch regressions:

```bash
# Run each fuzzer for 30 seconds
for target in fuzz_strings fuzz_quote_arg fuzz_wide_string fuzz_wide_builder fuzz_from_wide; do
    cargo +nightly fuzz run $target -- -max_total_time=30
done
```

## Troubleshooting

### "error: could not find `fuzz` in registry"
Install cargo-fuzz:
```bash
cargo install cargo-fuzz
```

### "error: the `-Z` flag is only accepted on nightly"
Switch to nightly:
```bash
rustup default nightly
```

### Out of memory
The fuzz targets limit input size, but you can also use:
```bash
cargo +nightly fuzz run fuzz_target -- -rss_limit_mb=2048
```

