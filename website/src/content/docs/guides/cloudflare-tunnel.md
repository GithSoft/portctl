---
title: Cloudflare Tunnel Ingress
description: Automated edge ingress, DNS creation, and zero-downtime daemon reloads via portctl expose.
---

This guide explains how `portctl` automates Cloudflare Tunnel ingress, the prerequisites required to use `portctl expose`, and security best practices.

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
credentials-file: ~/.cloudflared/<YOUR-TUNNEL-ID>.json

ingress:
  - service: http_status:404
```

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
  ```

---

## How `portctl expose` Works

Once your baseline tunnel is running, expose any local application with:

```bash
portctl expose my-app -d myapp.example.com
```

### The 6 Automated Steps

1. **Pre-flight Validation**: Verifies that `myapp.example.com` complies with RFC 1123 DNS standards.
2. **Security Checks**: Verifies that the service's port is not a blacklisted sensitive port (SSH `22`, PostgreSQL `5432`, Redis `6379`, Ollama `11434`, etc.).
3. **DNS Route Automation**: Runs `cloudflared tunnel route dns <tunnel-name> myapp.example.com` to create the CNAME record in Cloudflare DNS.
4. **Ingress Rule Insertion**: Locates the catch-all `http_status:404` rule and inserts the new route immediately before it.
5. **Syntax Validation**: Runs `cloudflared tunnel ingress validate` to prevent syntax corruption.
6. **Daemon Reload**: Triggers a zero-downtime configuration reload.

### Revoking an Ingress Route

When testing is finished, revoke the public route:

```bash
portctl unexpose my-app
```
