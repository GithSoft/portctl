# Cloudflare Tunnel Ingress Guide 🌐

This guide explains how `portctl` automates Cloudflare Tunnel ingress, the prerequisites required to use `portctl expose`, and security best practices.

---

## 📑 Table of Contents

- [Overview & Architecture](#overview--architecture)
- [One-Time Prerequisites & Setup](#one-time-prerequisites--setup)
  - [Step 1: Install cloudflared CLI](#step-1-install-cloudflared-cli)
  - [Step 2: Authenticate with Cloudflare](#step-2-authenticate-with-cloudflare)
  - [Step 3: Create a Named Tunnel](#step-3-create-a-named-tunnel)
  - [Step 4: Create Baseline Configuration](#step-4-create-baseline-configuration)
  - [Step 5: Install & Run Daemon Service](#step-5-install--run-daemon-service)
- [How `portctl expose` Works](#how-portctl-expose-works)
- [Configuration Path Resolution](#configuration-path-resolution)
- [Security Guardrails & Blacklist](#security-guardrails--blacklist)
- [Troubleshooting & FAQs](#troubleshooting--faqs)

---

## Overview & Architecture

**Cloudflare Tunnel** (part of Cloudflare Zero Trust) allows you to securely connect local development servers and microservices to the public Internet without:
- Opening ports on your home or office router (no NAT traversal).
- Exposing your public IP address.
- Managing local SSL/TLS certificates.

```
[Internet User]
       │
       ▼ (HTTPS: https://myapp.example.com)
[Cloudflare Edge] (DDoS Protection, WAF, SSL Termination)
       │
       ▼ (Outbound QUIC / HTTP2 Tunnel Connection)
[cloudflared Daemon] (Running locally on your machine)
       │
       ▼ (http://localhost:4001)
[Your Local App / Microservice]
```

With `portctl expose`, you don't need to manually edit YAML files, check port numbers, add DNS CNAME records, validate ingress rules, or kick the background daemon. **`portctl` handles all 10 steps in a single command.**

---

## One-Time Prerequisites & Setup

To use `portctl expose`, your machine needs an active Cloudflare Tunnel and the official `cloudflared` CLI.

### Step 1: Install cloudflared CLI

- **macOS** (Homebrew):
  ```bash
  brew install cloudflared
  ```
- **Linux** (Debian/Ubuntu):
  ```bash
  curl -fsSL https://pkg.cloudflare.com/cloudflare-main.gpg | sudo tee /usr/share/keyrings/cloudflare-main.gpg >/dev/null
  echo 'deb [signed-by=/usr/share/keyrings/cloudflare-main.gpg] https://pkg.cloudflare.com/cloudflared jammy main' | sudo tee /etc/apt/sources.list.d/cloudflared.list
  sudo apt-get update && sudo apt-get install cloudflared
  ```
- **Windows** (winget):
  ```powershell
  winget install --id Cloudflare.cloudflared
  ```

### Step 2: Authenticate with Cloudflare

Run the login command in your terminal:
```bash
cloudflared tunnel login
```
A browser window will open asking you to select the Cloudflare domain you want to associate with your machine. This downloads a certificate file to `~/.cloudflared/cert.pem`.

### Step 3: Create a Named Tunnel

Create a tunnel for your machine:
```bash
cloudflared tunnel create my-dev-tunnel
```
This outputs a **Tunnel ID** (UUID) and creates a credentials file at `~/.cloudflared/<tunnel-id>.json`.

### Step 4: Create Baseline Configuration

Create or edit `~/.cloudflared/config.yml`:

```yaml
tunnel: <YOUR-TUNNEL-ID>
credentials-file: /Users/<your-user>/.cloudflared/<YOUR-TUNNEL-ID>.json

ingress:
  - service: http_status:404
```

*(Note: Replace `<YOUR-TUNNEL-ID>` and user path with your actual details).*

### Step 5: Install & Run Daemon Service

Run `cloudflared` as a continuous background daemon:

- **macOS**:
  ```bash
  sudo cloudflared service install
  ```
- **Linux**:
  ```bash
  sudo cloudflared service install
  sudo systemctl start cloudflared
  sudo systemctl enable cloudflared
  ```

---

## How `portctl expose` Works

Once the prerequisites above are in place, exposing any registered service takes one command:

```bash
portctl expose my-app -d myapp.example.com
```

### Automation Flow:
1. **RFC 1123 Domain Validation**: Verifies hostname syntax to prevent injection attacks.
2. **Registry Lookup**: Retrieves the allocated port for `my-app` (e.g. `4001`).
3. **Blacklist Guardrail**: Checks if the port is a sensitive database or debugger port.
4. **Ingress Rule Insertion**: Inserts `{ hostname: myapp.example.com, service: http://localhost:4001 }` right before the catch-all `404` rule.
5. **Pre-flight Validation**: Runs `cloudflared tunnel --config <tmp> ingress validate` in a temporary sandbox (`0600` permissions) to ensure zero syntax errors.
6. **Atomic POSIX Write**: Safely updates `config.yml`.
7. **DNS CNAME Creation**: Calls `cloudflared tunnel route dns <tunnel-id> myapp.example.com` on Cloudflare Edge.
8. **Daemon Kickstart**: Triggers a sub-second zero-downtime daemon reload:
   - macOS: `launchctl kickstart -k system/com.cloudflare.cloudflared`
   - Linux: `systemctl restart cloudflared`
9. **Registry Metadata Sync**: Persists the public URL in your local port registry.

---

## Configuration Path Resolution

`portctl` looks for your Cloudflare Tunnel config file in the following order:

1. **`$PORTCTL_TUNNEL_CONFIG`**: An explicit environment variable pointing to any custom file path.
2. **`~/.cloudflared/config.yml`**: Standard user configuration path.
3. **`/etc/cloudflared/config.yml`**: Standard system-wide daemon path.

To specify a custom tunnel file, add this to your shell profile (`~/.zshrc` or `~/.bashrc`):
```bash
export PORTCTL_TUNNEL_CONFIG="/path/to/my-tunnel-config.yml"
```

---

## Security Guardrails & Blacklist

To prevent accidental exposure of internal infrastructure to the public Internet, `portctl` enforces a **Sensitive Port Blacklist**:

| Port | Service | Default Ingress Policy |
| :--- | :--- | :--- |
| **22** | SSH | 🚫 BLOCKED |
| **80 / 443** | HTTP / HTTPS (Reserved) | 🚫 BLOCKED |
| **2375** | Docker Daemon API | 🚫 BLOCKED |
| **3306** | MySQL | 🚫 BLOCKED |
| **5432** | PostgreSQL | 🚫 BLOCKED |
| **6379** | Redis | 🚫 BLOCKED |
| **8384** | Syncthing Admin UI | 🚫 BLOCKED |
| **9222** | Chrome Remote Debugging | 🚫 BLOCKED |
| **11434** | Ollama LLM API | 🚫 BLOCKED |
| **27017** | MongoDB | 🚫 BLOCKED |

### Bypassing Blacklist (When Intentional)
If you understand the security implications and genuinely need to expose one of these ports, append the `--force` flag:

```bash
portctl expose redis -d redis.example.com --force
```

---

## Troubleshooting & FAQs

### Q: `cloudflared: command not found`
**Fix**: Install the `cloudflared` CLI following [Step 1](#step-1-install-cloudflared-cli).

### Q: `Cloudflare Tunnel config file not found`
**Fix**: Create `~/.cloudflared/config.yml` following [Step 4](#step-4-create-baseline-configuration) or set `$PORTCTL_TUNNEL_CONFIG`.

### Q: Does `portctl` work without Cloudflare?
**Yes!** All core port management features (`portctl list`, `get`, `assign`, `check`, `run`, `env`) function completely offline without Cloudflare or an Internet connection. Only `portctl expose` interacts with Cloudflare.

### Q: Can I restrict access to my team only?
**Yes.** Once your tunnel is active, you can navigate to the [Cloudflare Zero Trust Dashboard](https://one.dash.cloudflare.com/) and configure an **Access Application** for `myapp.example.com` to require Google, GitHub, or Okta SSO login before traffic reaches your local machine.
