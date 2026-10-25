# Contributing to portctl (Rust Edition) 🦀

Thank you for contributing to `portctl`! We welcome contributions to make local port management and edge ingress fast, robust, and universal.

---

## 🛠️ Development Setup

1. **Prerequisites**: Install Rust 1.75+ via [rustup](https://rustup.rs).
2. **Clone & Build**:
   ```bash
   git clone https://github.com/tackelua/portctl.git
   cd portctl
   cargo build
   ```
3. **Run Tests**:
   ```bash
   cargo test
   ```
4. **Install Locally**:
   ```bash
   cargo install --path .
   ```

---

## 📐 Architecture

- `src/main.rs`: CLI commands definition via `clap`.
- `src/detector.rs`: Polyglot framework manifest auto-detector.
- `src/registry.rs`: Partitioned JSON storage with atomic writes and advisory file locks.
- `src/allocator.rs`: Port range conflict resolver.
- `src/probe.rs`: Non-blocking socket probe and PID inspector.
- `src/tunnel.rs`: Cloudflare Tunnel Ingress YAML generator with RFC 1123 validation.
- `src/constants.rs`: XDG Base Directory path resolver and security guardrails.
