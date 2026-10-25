//! # portctl
//!
//! `portctl` is a lightweight, zero-daemon, local-first Port Manager and automated
//! Edge Ingress CLI written in pure Rust.
//!
//! ## Core Architecture
//! - [`detector`]: Auto-detects project identities from polyglot manifests (`package.json`, `Cargo.toml`, etc.).
//! - [`registry`]: Manages canonical and machine-local port assignments with atomic POSIX file persistence.
//! - [`allocator`]: Handles conflict resolution and auto-allocation in the dynamic port range (4000-4999).
//! - [`probe`]: Real-time socket status verification using OS TCP listener binding and `lsof` process diagnostics.
//! - [`tunnel`]: Automates Cloudflare Tunnel ingress configuration and daemon kickstarts.
//! - [`runner`]: Injects `$PORT` into child processes for zero-boilerplate 12-factor apps.
//! - [`killer`]: Process hunter for gracefully terminating processes occupying ports.
//! - [`doctor`]: System diagnostics for auditing permissions, hijacks, ghost entries, and ingress rules.

pub mod allocator;
pub mod constants;
pub mod detector;
pub mod doctor;
pub mod killer;
pub mod probe;
pub mod registry;
pub mod runner;
pub mod tunnel;
