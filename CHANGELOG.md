# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [2.1.0] - 2026-10-25

### Added
- **Process Hunter (`portctl kill`)**: Gracefully terminate processes occupying ports using progressive signal escalation (`SIGTERM` -> 3s -> `SIGKILL`), with sensitive port guardrails and instant permission error detection.
- **System Diagnostics (`portctl doctor`)**: Full diagnostic suite auditing rogue listening ports, ghost entries, dangling ingress rules, and file permissions, with `--fix` automated remediation and `--json` machine-readable output.
- **Tunnel Ingress Revocation (`portctl unexpose`)**: Automated cleanup of Cloudflare Tunnel ingress rules, pre-flight syntax validation, zero-downtime daemon kickstart, and registry synchronization.
- **Shell Auto-Completions (`portctl completion`)**: Dynamic completion generator for Bash, Zsh, Fish, PowerShell, and Elvish via `clap_complete`.
- **Advisory File Locking (`fd-lock`)**: Cross-process mutation safety using a dedicated `~/.config/portctl/.lock` file, avoiding inode replacement issues during atomic file writes while keeping read operations completely lock-free (< 3ms).
- **Extensible `TunnelProvider` Trait**: Modular abstraction layer for multi-tunnel backends.

---

## [2.0.0] - 2026-10-25

### Added
- **Pure Rust Core Engine**: Completely rewritten from the ground up in Rust for sub-millisecond execution (< 3ms) and zero runtime dependencies.
- **Polyglot Auto-Detection**: Automatically identifies project names from `package.json`, `Cargo.toml`, `pyproject.toml`, `go.mod`, `pom.xml`, `composer.json`, `mix.exs`, and Git origin remotes.
- **Zero-Daemon Architecture**: Operates directly on partitioned local storage without requiring a continuous background daemon process.
- **Active Socket Probing**: Real-time socket status verification using OS TCP listener binding and `lsof` process diagnostics (PID, command, user).
- **Subprocess Runner**: `portctl run -- <command>` transparently injects `$PORT` into child process environments.
- **Cloudflare Edge Ingress**: 1-click Cloudflare Tunnel exposure (`portctl expose`) with RFC 1123 domain validation, ingress YAML generation, `cloudflared` configuration validation, and daemon kickstart.
- **POSIX Atomic File I/O**: Secure state persistence with strict `0600` file permissions to prevent file corruption during power failures or unexpected crashes.
- **XDG Base Directory Compliance**: Standardized configuration and registry storage in `~/.config/portctl/`.
- **Cross-Platform Support**: Tier-1 support for Linux and macOS.
