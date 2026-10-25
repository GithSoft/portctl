---
title: Architecture & Concurrency Locking
description: Pure Rust design, XDG Base Directory standard, POSIX advisory locks, and atomic operations.
---

`portctl` is engineered from the ground up to be lightweight, zero-daemon, sub-millisecond, and resilient against system crashes or concurrent writes.

## The Zero-Daemon Principle

Traditional port and ingress managers run continuous background daemons, eating memory and introducing points of failure.

`portctl` requires **zero background daemon**. It is a pure static binary that executes in `< 3ms`:

```
[CLI Invocation]
       │
       ▼ (1) Resolve project identity via polyglot detection
[Identity Resolver]
       │
       ▼ (2) Acquire POSIX advisory lock (~/.config/portctl/.lock)
[fd-lock Mutex]
       │
       ▼ (3) Read registry (~/.config/portctl/ports.json)
[Registry State]
       │
       ▼ (4) Probe real-time socket (TCP bind / lsof probe)
[Socket Probe]
       │
       ▼ (5) Atomic rename & release lock
[State Persisted]
```

---

## Concurrency Safety & POSIX Advisory Locks

When multiple terminal tabs, IDE tasks, or AI coding agents run concurrently, simultaneous mutations to `ports.json` could cause corruption.

`portctl` utilizes **dedicated advisory file locks** (`fd-lock`) on a separate lockfile:

1. **Dedicated Lock File**: `~/.config/portctl/.lock`. Locking a dedicated file avoids invalidating the file descriptor during atomic renames.
2. **Atomic Writes (`rename(2)`)**: When saving registry data, `portctl` writes to a temporary file on the same filesystem partition and issues an atomic `rename` call. Even in the event of power loss, the file is never left partially written.
3. **Strict File Permissions**: Data files are restricted to mode `0600` (read/write only by the owning user), and directories are set to `0700`.

---

## XDG Base Directory Standard

Configuration and machine state are separated according to the XDG Base Directory Specification:

- **Config**: `~/.config/portctl/config.json`
- **Port Registry**: `~/.config/portctl/ports.json`
- **Machine Allocations**: `~/.config/portctl/registry/machines/`
- **Lockfile**: `~/.config/portctl/.lock`
