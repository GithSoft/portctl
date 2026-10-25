---
title: CLI Command Reference
description: Complete command-line flag and argument reference for portctl.
---

`portctl` commands and options cheat sheet.

## Global Flags

| Flag | Description |
| :--- | :--- |
| `-h, --help` | Print help information |
| `-V, --version` | Print version information |
| `--json` | Output response in structured JSON format |

---

## Commands

### `portctl list`
Lists all assigned ports and real-time listening socket statuses.

```bash
portctl list [--json]
```

### `portctl get [app]`
Resolves the deterministic port assigned to a project. If `[app]` is omitted, automatically detects from `package.json`, `Cargo.toml`, etc.

```bash
portctl get [app]
```

### `portctl run -- <command...>`
Executes any arbitrary command with `$PORT` injected into the environment.

```bash
portctl run -- npm run dev
portctl run -- uvicorn main:app --port $PORT
```

### `portctl check [target]`
Probes socket status and retrieves process ownership (PID, process name, memory RSS).

```bash
portctl check           # Check current project
portctl check 3000      # Check specific port
portctl check my-app    # Check by service name
```

### `portctl assign <app>`
Registers an application with fixed or auto-incremented port allocations.

```bash
portctl assign redis -f 6379 -s local --desc "Redis Cache Server"
portctl assign web-frontend -d frontend.example.com
```

### `portctl kill <target>`
Terminates processes occupying a socket using progressive SIGTERM -> SIGKILL escalation.

```bash
portctl kill 3000
portctl kill web-frontend
portctl kill 5432 --force   # Override protected infrastructure port guardrail
```

### `portctl expose <app>`
Automates Cloudflare Tunnel ingress configuration and daemon reload.

```bash
portctl expose my-app -d myapp.example.com
```

### `portctl unexpose <app>`
Revokes Cloudflare Tunnel ingress routes and reloads the daemon.

```bash
portctl unexpose my-app
```

### `portctl doctor`
Performs system health audit on permissions, lockfiles, and dangling ingress rules.

```bash
portctl doctor          # Audit only
portctl doctor --fix    # Automatically repair anomalies
portctl doctor --json   # Machine-readable report
```

### `portctl env`
Generates `.env` variables or shell export statements.

```bash
portctl env -w          # Write PORT into .env or .env.local
portctl env -e          # Output 'export PORT=...'
```

### `portctl completion <shell>`
Generates shell autocompletion script.

```bash
portctl completion zsh
portctl completion bash
portctl completion fish
```
