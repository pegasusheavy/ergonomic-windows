# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-12-29

### Added

- **Core Modules**
  - `string` - UTF-8/UTF-16 string conversions with `WideString` type
  - `handle` - RAII handle management with `OwnedHandle` and `BorrowedHandle`
  - `error` - Unified error handling with Windows error integration

- **System Modules**
  - `process` - Process creation, enumeration, and management
  - `registry` - Type-safe Windows Registry access
  - `fs` - File system operations and attributes
  - `window` - Window creation and message handling
  - `thread` - Thread creation and synchronization primitives (Mutex, Event, Semaphore)
  - `mem` - Virtual memory allocation and management
  - `console` - Console I/O with colors and text attributes
  - `env` - Environment variable operations
  - `pipe` - Anonymous and named pipe support
  - `time` - System time, performance counters, and stopwatch
  - `module` - DLL loading and symbol resolution
  - `sysinfo` - System information queries
  - `security` - Token and privilege management

- **UI Modules**
  - `controls` - Win32 Common Controls (Button, Edit, Label, ListBox, ComboBox, ProgressBar)
  - `d2d` - Direct2D/DirectWrite graphics and text rendering
  - `webview` - WebView2 integration (optional feature)
  - `xaml` - WinRT XAML types and utilities

- **Infrastructure**
  - Comprehensive documentation website
  - GitHub Actions CI/CD workflows
  - Benchmarking suite with Criterion
  - Memory profiling with dhat
  - Fuzzing support with cargo-fuzz

### Security

- All Windows API calls wrapped with safe Rust abstractions
- RAII patterns for automatic resource cleanup
- Input validation on all public APIs

[0.1.0]: https://github.com/pegasusheavy/ergonomic-windows/releases/tag/v0.1.0

