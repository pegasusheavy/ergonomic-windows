# ergonomic-windows

[![Crates.io](https://img.shields.io/crates/v/ergonomic-windows.svg)](https://crates.io/crates/ergonomic-windows)
[![Documentation](https://docs.rs/ergonomic-windows/badge.svg)](https://docs.rs/ergonomic-windows)
[![License](https://img.shields.io/crates/l/ergonomic-windows.svg)](LICENSE-MIT)
[![CI](https://github.com/pegasusheavy/ergonomic-windows/actions/workflows/ci.yml/badge.svg)](https://github.com/pegasusheavy/ergonomic-windows/actions/workflows/ci.yml)
[![Security Audit](https://github.com/pegasusheavy/ergonomic-windows/actions/workflows/security.yml/badge.svg)](https://github.com/pegasusheavy/ergonomic-windows/actions/workflows/security.yml)

Ergonomic, safe Rust wrappers for Windows APIs — handles, processes, registry, file system, and UTF-16 strings with zero-cost abstractions.

[📖 Documentation](https://pegasusheavy.github.io/ergonomic-windows/) | [📚 API Reference](https://docs.rs/ergonomic-windows)

## Features

- 🛡️ **Safe by Default** — RAII wrappers automatically manage Windows handles
- 🔤 **Zero-Cost Strings** — Small string optimization for UTF-16 conversions (no heap allocation for strings ≤22 chars)
- ⚡ **High Performance** — Object pooling and optimized allocations for high-throughput scenarios
- 🎯 **Ergonomic API** — Fluent builders and idiomatic Rust patterns
- 📝 **Rich Error Handling** — Typed errors with Windows error code context
- 🧪 **Well Tested** — 80+ tests covering edge cases, Unicode, and stress scenarios

## Modules

| Module | Description |
|--------|-------------|
| `string` | UTF-8 ↔ UTF-16 conversion with small string optimization and object pooling |
| `handle` | RAII wrappers for Windows `HANDLE` with automatic cleanup |
| `process` | Process creation, management, and querying |
| `registry` | Windows Registry read/write with type-safe values |
| `fs` | Windows-specific file system operations |
| `window` | Window creation and message handling |
| `error` | Rich error types with Windows error code support |

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
ergonomic-windows = "0.1"
```

Or use cargo:

```bash
cargo add ergonomic-windows
```

## Quick Start

```rust
use ergonomic_windows::prelude::*;

fn main() -> Result<()> {
    // Spawn a process
    let exit_code = Command::new("cmd.exe")
        .args(["/c", "echo", "Hello from Rust!"])
        .no_window()
        .run()?;
    println!("Exit code: {}", exit_code);

    // Read from the registry
    let key = Key::open(
        RootKey::CURRENT_USER,
        r"Software\Microsoft\Windows\CurrentVersion\Explorer",
        Access::READ,
    )?;

    if let Ok(value) = key.get_value("ShellState") {
        println!("ShellState: {:?}", value);
    }

    Ok(())
}
```

## Usage Examples

### String Conversion

Convert between Rust UTF-8 strings and Windows UTF-16 strings:

```rust
use ergonomic_windows::string::{to_wide, from_wide, WideString};

// Basic conversion
let wide = to_wide("Hello, Windows! 🎉");
let back = from_wide(&wide)?;
assert_eq!(back, "Hello, Windows! 🎉");

// WideString for Windows API calls
let ws = WideString::new("C:\\Windows\\System32");
// ws.as_pcwstr() returns PCWSTR for Windows APIs

// Small strings are stored inline (no heap allocation!)
let small = WideString::new("Hello");
assert!(small.is_inline()); // true for strings ≤22 UTF-16 chars
```

#### High-Throughput String Pool

For scenarios converting many strings, use the object pool to reuse buffers:

```rust
use ergonomic_windows::string::WideStringPool;

let mut pool = WideStringPool::new();

for filename in &["file1.txt", "file2.txt", "file3.txt"] {
    let wide = pool.get(filename);
    // Use wide.as_pcwstr() with Windows APIs
    pool.put(wide); // Return buffer for reuse
}
```

### Handle Management

Windows handles are automatically closed when dropped:

```rust
use ergonomic_windows::handle::OwnedHandle;
use ergonomic_windows::fs::OpenOptions;

// Open a file - handle is automatically managed
let handle = OpenOptions::new()
    .read(true)
    .open("file.txt")?;

// Clone handles safely
let cloned = handle.try_clone()?;

// Handles close automatically when dropped
```

### Process Management

Create and manage Windows processes:

```rust
use ergonomic_windows::process::{Command, Process, ProcessAccess};

// Spawn a process
let process = Command::new("notepad.exe")
    .arg("document.txt")
    .current_dir("C:\\Users\\Public")
    .env("MY_VAR", "value")
    .spawn()?;

println!("Started process with PID: {}", process.pid());

// Wait with timeout
use std::time::Duration;
match process.wait_timeout(Some(Duration::from_secs(5))) {
    Ok(exit_code) => println!("Exited with: {}", exit_code),
    Err(_) => {
        process.terminate(1)?;
        println!("Process terminated");
    }
}

// Open existing process
let current = Process::open(process.pid(), ProcessAccess::QUERY)?;
println!("Is running: {}", current.is_running()?);
```

### Registry Access

Read and write Windows Registry values:

```rust
use ergonomic_windows::registry::{Key, RootKey, Access, Value};

// Read system information
let key = Key::open(
    RootKey::LOCAL_MACHINE,
    r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
    Access::READ,
)?;

if let Ok(Value::String(name)) = key.get_value("ProductName") {
    println!("Windows Version: {}", name);
}

// Write application settings
let app_key = Key::create(
    RootKey::CURRENT_USER,
    r"Software\MyApp\Settings",
    Access::ALL,
)?;

app_key.set_value("Volume", &Value::dword(75))?;
app_key.set_value("Username", &Value::string("Alice"))?;
app_key.set_value("RecentFiles", &Value::MultiString(vec![
    "doc1.txt".into(),
    "doc2.txt".into(),
]))?;

// Enumerate keys and values
for subkey in app_key.subkeys()? {
    println!("Subkey: {}", subkey);
}
```

### File System Operations

Windows-specific file operations:

```rust
use ergonomic_windows::fs::{
    get_attributes, set_attributes, FileAttributes,
    exists, is_dir, is_file, delete_file,
    move_file, move_file_with_options, MoveOptions,
    get_system_directory, get_temp_directory,
};

// Check file attributes
let attrs = get_attributes("C:\\Windows")?;
assert!(attrs.is_directory());

// Get system paths
let system_dir = get_system_directory()?;
let temp_dir = get_temp_directory()?;
println!("System: {:?}", system_dir);
println!("Temp: {:?}", temp_dir);

// Move with options
move_file_with_options(
    "old_path.txt",
    "new_path.txt",
    MoveOptions::new().replace().allow_copy(),
)?;
```

### Window Creation

Create windows and handle messages:

```rust
use ergonomic_windows::window::{WindowBuilder, MessageHandler, Message};
use ergonomic_windows::prelude::*;

struct MyHandler;

impl MessageHandler for MyHandler {
    fn on_create(&mut self, hwnd: windows::Win32::Foundation::HWND) -> Result<()> {
        println!("Window created!");
        Ok(())
    }

    fn on_destroy(&mut self, _hwnd: windows::Win32::Foundation::HWND) -> Result<()> {
        std::process::exit(0);
    }
}

let window = WindowBuilder::new()
    .title("My Window")
    .size(800, 600)
    .style(Style::OVERLAPPEDWINDOW)
    .build(MyHandler)?;

window.show(ShowCommand::Show);
Message::run_loop();
```

## Safety

This crate uses `unsafe` code to interface with Windows APIs. All unsafe blocks are:

- **Documented** with safety invariants
- **Minimal** in scope
- **Audited** for correctness

The public API is entirely safe Rust. Handles are managed via RAII, preventing:
- Use-after-close bugs
- Double-close bugs
- Handle leaks

## Performance

The crate is optimized for performance:

| Optimization | Impact |
|--------------|--------|
| Small String Optimization | Zero allocation for strings ≤22 chars |
| Object Pooling | Reusable buffers for high-throughput scenarios |
| Pre-allocated Buffers | Reduced allocations in string builders |
| Inline Hints | Hot paths marked for inlining |

Benchmark results (typical):
- `WideString::new("Hello")`: **0 allocations**, ~12ns
- `to_wide` / `from_wide`: **1 allocation** each
- String pool reuse: **0 allocations** after warmup

## Minimum Supported Rust Version

This crate requires **Rust 1.70** or later.

## Contributing

Contributions are welcome! Please see our [Contributing Guidelines](CONTRIBUTING.md).

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Development

```bash
# Run tests
cargo test

# Run benchmarks
cargo bench

# Run clippy
cargo clippy --all-targets

# Build docs
cargo doc --open
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Acknowledgments

Built with the excellent [windows-rs](https://github.com/microsoft/windows-rs) crate from Microsoft.

---

<p align="center">
  Made with ❤️ by <a href="https://github.com/pegasusheavy">Pegasus Heavy Industries</a>
</p>

