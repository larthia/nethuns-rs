# Contributing to Nethuns-rs

Thank you for your interest in contributing to Nethuns-rs! This document provides guidelines and information for contributors.

## Table of Contents

- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Code Style](#code-style)
- [Making Changes](#making-changes)
- [Testing](#testing)
- [Pull Request Process](#pull-request-process)
- [Architecture Overview](#architecture-overview)

## Getting Started

### Prerequisites

- Rust 1.70 or later (stable toolchain)
- For Linux backend development:
  - libbpf-dev and libxdp-dev (for AF_XDP)
  - netmap headers and kernel module (for netmap)
  - DPDK development libraries (for DPDK)
- libpcap development headers

### Fork and Clone

1. Fork the repository on GitHub
2. Clone your fork:

   ```bash
   git clone https://github.com/YOUR_USERNAME/nethuns-rs.git
   cd nethuns-rs
   ```

3. Add the upstream remote:

   ```bash
   git remote add upstream https://github.com/leonardogiovannoni/nethuns-rs.git
   ```

## Development Setup

### Building

```bash
# Basic build (pcap only)
cargo build

# Build with all features (Linux only)
cargo build --features "af_xdp,netmap,dpdk"

# Build examples
cargo build --examples
```

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with specific features
cargo test --features af_xdp
```

### Documentation

```bash
# Generate and view documentation
cargo doc --open
```

## Code Style

### Rust Guidelines

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `rustfmt` for formatting:

  ```bash
  cargo fmt
  ```

- Run `clippy` for linting:

  ```bash
  cargo clippy --all-features
  ```

### Naming Conventions

- Modules: `snake_case`
- Types: `PascalCase`
- Functions: `snake_case`
- Constants: `SCREAMING_SNAKE_CASE`

### Documentation

- All public items must have documentation
- Use `///` for item documentation
- Include examples in documentation when helpful

### Example

```rust
/// Represents a network packet buffer.
///
/// # Examples
///
/// ```rust
/// let buffer = PacketBuffer::new(2048);
/// assert_eq!(buffer.capacity(), 2048);
/// ```
pub struct PacketBuffer {
    // ...
}
```

## Making Changes

### Branch Naming

- `feature/description` - New features
- `fix/description` - Bug fixes
- `docs/description` - Documentation changes
- `refactor/description` - Code refactoring

### Commit Messages

Follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

```
type(scope): description

[optional body]

[optional footer]
```

Types:

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `style`: Formatting
- `refactor`: Code restructuring
- `test`: Adding tests
- `chore`: Maintenance

Examples:

```
feat(af_xdp): add zero-copy mode support
fix(pcap): handle timeout correctly on macOS
docs(api): add Socket trait documentation
```

## Testing

### Unit Tests

Add unit tests in the same file as the code:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_allocation() {
        // ...
    }
}
```

### Integration Tests

Add integration tests in `tests/`:

```rust
// tests/pcap_integration.rs
use nethuns_rs::pcap::{Sock, PcapFlags};

#[test]
fn test_pcap_create() {
    // ...
}
```

### Running Examples

Test examples on actual hardware:

```bash
# Requires sudo and a network interface
sudo cargo run --example meter -- -i eth0 pcap
```

## Pull Request Process

1. **Create a branch** from `main`:

   ```bash
   git checkout -b feature/my-feature
   ```

2. **Make your changes** and commit:

   ```bash
   git add .
   git commit -m "feat(scope): description"
   ```

3. **Push to your fork**:

   ```bash
   git push origin feature/my-feature
   ```

4. **Open a Pull Request** on GitHub

5. **PR Checklist**:
   - [ ] Code compiles without warnings
   - [ ] All tests pass
   - [ ] Documentation is updated
   - [ ] Changelog is updated (if applicable)
   - [ ] Code is formatted with `rustfmt`
   - [ ] Code passes `clippy`

6. **Review Process**:
   - Wait for review from maintainers
   - Address any requested changes
   - Once approved, the PR will be merged

## Architecture Overview

### Module Structure

```
src/
├── lib.rs          # Library root, feature flags
├── api/
│   └── mod.rs      # Core traits (Socket, Context, etc.)
├── errors.rs       # Error types
├── pcap.rs         # pcap backend
├── af_xdp/
│   ├── mod.rs      # AF_XDP socket implementation
│   └── wrapper.rs  # Low-level XDP bindings
├── netmap/
│   └── mod.rs      # netmap socket implementation
└── dpdk/
    ├── mod.rs      # DPDK socket implementation
    └── wrapper.rs  # Low-level DPDK bindings
```

### Core Traits

```
Socket          # Network I/O operations
    ├── recv()
    ├── send()
    ├── flush()
    └── create()

Context         # Buffer management
    ├── pool_id()
    ├── unsafe_buffer()
    └── release()

Payload         # Smart pointer to packet data
    └── Deref<Target = [u8]>

Token           # Transferable buffer ownership
    └── consume() -> Payload
```

### Adding a New Backend

1. Create a new module (e.g., `src/io_uring/mod.rs`)
2. Implement the `Socket`, `Context`, `Metadata`, and `Flags` traits
3. Add feature flag in `Cargo.toml`
4. Add conditional compilation in `src/lib.rs`
5. Add tests and examples
6. Update documentation

### Memory Safety Considerations

- Use `unsafe` sparingly and document safety invariants
- Prefer safe Rust when possible
- Use `ManuallyDrop` for custom drop behavior
- Validate buffer ownership with `pool_id()` checks

## Questions?

- Open an issue for bugs or feature requests
- Start a discussion for general questions
- Check existing issues before creating new ones

Thank you for contributing!
