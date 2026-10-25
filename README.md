# portctl ⚡

[![CI](https://github.com/tackelua/portctl/actions/workflows/ci.yml/badge.svg)](https://github.com/tackelua/portctl/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Language: Rust](https://img.shields.io/badge/Language-Rust-orange.svg)](https://www.rust-lang.org/)

**portctl** is a lightweight, zero-daemon, local-first Port Manager and automated Edge Ingress CLI written in pure Rust.

It prevents port collisions across microservices, eliminates hardcoded port numbers, auto-detects project identities, and connects local services to the Internet via Cloudflare Tunnels in a single command.

---

## ✨ Features

- 🦀 **Pure Rust & High Performance**: Sub-millisecond execution, minimal memory footprint, zero daemon or runtime dependencies.
- 🔍 **Polyglot Auto-Detection**: Automatically identifies project names from `package.json`, `Cargo.toml`, `pyproject.toml`, `go.mod`, `pom.xml`, `composer.json`, `mix.exs`, or Git remotes.
- 🔌 **Zero Boilerplate Execution**: Run any service with `portctl run -- <command>` to inject `$PORT` into the environment seamlessly.
- 🛡️ **Active Socket Probing**: Real-time port inspection using OS socket binding and `lsof` process diagnostics (PID, command, user).
- 🌐 **Automated Edge Ingress**: 1-click Cloudflare Tunnel ingress configuration with ingress validation, DNS routing, and daemon reloading.
- 🔒 **Security Guardrails**: Blocks direct exposure of sensitive internal ports (SSH, MySQL, Postgres, Redis, Docker, Chrome DevTools) unless explicitly overridden with `--force`.
- 📁 **XDG Base Directory Compliant**: Stores configuration and registry data safely at `~/.config/portctl/` with POSIX atomic writes and `0600` permissions.

---

## 🚀 Installation

### Using Cargo (Recommended)

```bash
cargo install --git https://github.com/tackelua/portctl
```

### From Source

```bash
git clone https://github.com/tackelua/portctl.git
cd portctl
cargo build --release
sudo cp target/release/portctl /usr/local/bin/
```

---

## 📖 CLI Usage

### 1. View Port Registry & Process Statuses

```bash
portctl list
```
Displays an interactive table of all registered ports, their scope, active socket status, and associated Cloudflare Tunnel domains.

To output JSON:
```bash
portctl list --json
```

### 2. Auto-Allocate or Query Port

From any project directory:
```bash
# Returns the assigned port for the current project (auto-detected)
portctl get

# Or specify by name:
portctl get my-app
```

### 3. Run Application with Injected Port

Execute any development server or background script with `$PORT` automatically injected:

```bash
# Node.js
portctl run -- npm run dev

# Python / FastAPI
portctl run -- uvicorn main:app --port $PORT

# Go / Rust
portctl run -- ./server
```

### 4. Assign Fixed or Custom Port

```bash
# Register a custom port for an internal database or static service
portctl assign redis -f 6379 -s local --desc "Redis Cache Server"

# Allocate next available port for an app with a public domain
portctl assign my-frontend -d app.example.com
```

### 5. Inspect Socket & Process

```bash
# Inspect current directory's app
portctl check

# Inspect specific port number
portctl check 4001

# Inspect service by name
portctl check word-frequency-analyzer
```

### 6. Expose Service via Cloudflare Tunnel

```bash
portctl expose my-app -d myapp.example.com
```
Validates ingress syntax, creates DNS route, updates tunnel configuration, and reloads the local cloudflared daemon.

### 7. Sync Environment Variables

```bash
# Write or update PORT in .env (or .env.local for Next.js)
portctl env -w

# Output shell export statement
portctl env -e
# eval $(portctl env -e)
```

---

## 🛠️ App Integration Guide

### Node.js / Express / Fastify

No external libraries required! Simply rely on standard environment variables:

```javascript
const PORT = process.env.PORT || 4001;

app.listen(PORT, () => {
  console.log(`Server listening on http://localhost:${PORT}`);
});
```

Run with `portctl run -- npm start` or run `portctl env -w` to persist `PORT` into your `.env` file.

### Python (FastAPI / Flask)

```python
import os
import uvicorn

port = int(os.environ.get("PORT", 8000))

if __name__ == "__main__":
    uvicorn.run("main:app", host="0.0.0.0", port=port)
```

---

## 🤝 Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) and [SECURITY.md](SECURITY.md) before opening a pull request.

```bash
cargo fmt --all -- --check
cargo clippy -- -D warnings
cargo test
```

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).
