# Contributing to ergonomic-windows

Thank you for your interest in contributing! This document provides guidelines and information about contributing to the project.

## Code of Conduct

Please be respectful and constructive in all interactions.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone git@github.com:YOUR_USERNAME/ergonomic-windows.git`
3. Create a branch: `git checkout -b feature/your-feature-name`
4. Make your changes
5. Run tests: `cargo test`
6. Run lints: `cargo clippy --all-targets --all-features`
7. Format code: `cargo fmt`
8. Commit your changes
9. Push to your fork
10. Open a Pull Request

## Development Setup

### Prerequisites

- Windows 10/11
- Rust stable toolchain (MSRV: 1.70.0)
- Git

### Building

```bash
# Debug build
cargo build

# Release build
cargo build --release
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run benchmarks
cargo bench

# Run with memory profiling
cargo run --bin memory-profile --features dhat-heap
```

### Fuzzing (requires nightly)

```bash
cd fuzz
cargo +nightly fuzz run fuzz_strings -- -max_total_time=60
```

## Pull Request Guidelines

### Before Submitting

- [ ] Code compiles without warnings
- [ ] All tests pass
- [ ] `cargo clippy` reports no warnings
- [ ] `cargo fmt` has been run
- [ ] Documentation is updated if needed
- [ ] Commit messages are clear and descriptive

### PR Requirements

1. **One concern per PR**: Keep PRs focused on a single feature or fix
2. **Tests**: Include tests for new functionality
3. **Documentation**: Update docs for API changes
4. **Breaking changes**: Clearly document in PR description

## Code Style

- Follow Rust idioms and conventions
- Use `#[inline]` hints for hot paths
- Document all public APIs with examples
- Add safety comments to all `unsafe` blocks
- Prefer zero-allocation paths where possible

### Safety Guidelines

When working with `unsafe` code:

1. Document all safety requirements in `# Safety` sections
2. Add `// SAFETY:` comments explaining why the unsafe operation is sound
3. Minimize unsafe scope
4. Prefer safe abstractions over raw unsafe code

## Commit Messages

Use clear, descriptive commit messages:

```
feat: add WideString::with_capacity for pre-allocation

- Allows users to pre-allocate buffer capacity
- Reduces allocations in tight loops
- Includes documentation and tests
```

Prefixes:
- `feat:` - New feature
- `fix:` - Bug fix
- `docs:` - Documentation only
- `perf:` - Performance improvement
- `refactor:` - Code refactoring
- `test:` - Adding tests
- `ci:` - CI/CD changes
- `deps:` - Dependency updates

## Reporting Issues

When reporting bugs, please include:

1. Windows version
2. Rust version (`rustc --version`)
3. Crate version
4. Minimal reproduction code
5. Expected vs actual behavior

## Feature Requests

We welcome feature requests! Please:

1. Check existing issues first
2. Describe the use case
3. Propose an API design if possible
4. Explain why existing solutions don't work

## Questions

For questions:

1. Check the documentation
2. Search existing issues
3. Open a question issue if still unclear

## License

By contributing, you agree that your contributions will be licensed under the same dual MIT/Apache-2.0 license as the project.

