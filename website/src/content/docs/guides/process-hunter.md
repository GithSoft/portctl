---
title: Process Hunter (Graceful Termination)
description: Progressive SIGTERM to SIGKILL socket reclamation with system guardrails.
---

When developing microservices, zombie processes or abrupt terminal crashes frequently hold TCP sockets hostage, resulting in the dreaded `Error: listen EADDRINUSE :::3000`.

`portctl kill` provides a safe, progressive process hunter designed to release sockets without causing database corruption or data loss.

## Basic Usage

Terminate the process occupying a specific port or assigned service:

```bash
# Kill by port number
portctl kill 3000

# Kill by application name
portctl kill web-frontend
```

## Progressive Signal Escalation

Instead of blindly issuing `kill -9` (`SIGKILL`) which truncates database transactions and corrupts state files, `portctl` executes a 2-stage progressive escalation:

```
[Target Process Detected]
        │
        ▼ (Stage 1)
[Send SIGTERM] (Graceful termination signal)
        │
        ▼ (3,000ms Polling Loop)
Is process still alive?
 ├── NO  ──▶ [Success] Socket released cleanly!
 └── YES ──▶ (Stage 2)
             [Send SIGKILL] (Forced immediate kernel termination)
```

1. **Stage 1 (SIGTERM)**: Sends `SIGTERM` to the process. Most HTTP frameworks (Express, Fastify, Spring, Actix, Axum) intercept this signal to drain active client connections, flush logs, and close open database pools.
2. **Grace Period**: Polls the process table every 200ms for up to 3 seconds (`3000ms`).
3. **Stage 2 (SIGKILL Fallback)**: If the process hangs, enters a deadlock, or refuses to terminate after 3 seconds, `portctl` escalates to `SIGKILL` (`kill -9`) to guarantee that the port is freed.

---

## Security Guardrails

To protect against accidental system disruption, `portctl kill` enforces built-in safety boundaries:

- **System PID Protection**: Never kills `PID <= 1` (kernel/init/systemd/launchd).
- **Protected Ports**: Rejects killing critical infrastructure ports unless `--force` is explicitly provided:
  - SSH (`22`)
  - HTTP/HTTPS (`80`, `443`)
  - Databases (`5432`, `3306`, `27017`, `6379`)
  - Docker daemon (`2375`, `2376`)

```bash
# Protected port rejection
portctl kill 5432
# Output: Error: Port 5432 is a protected infrastructure port. Use --force to override.
```
