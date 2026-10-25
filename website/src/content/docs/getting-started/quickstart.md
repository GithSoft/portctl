---
title: Quickstart
description: Get up and running with portctl in under 2 minutes.
---

`portctl` eliminates hardcoded port numbers, prevents `EADDRINUSE` collisions, and automates public HTTPS ingress for local microservices.

## 1. Verify Installation

Check that `portctl` is present in your PATH:

```bash
portctl --version
# Output: portctl 2.1.0
```

## 2. Inspect Current Port Registry

Run `portctl list` to inspect all assigned ports, their scopes, and real-time socket statuses:

```bash
portctl list
```

## 3. Run Your Application

Instead of hardcoding `PORT=3000` or wrestling with `.env` files, run your development server through `portctl run`:

```bash
# Node.js / Next.js / Vite
portctl run -- npm run dev

# Python / FastAPI
portctl run -- uvicorn main:app --port $PORT

# Go / Rust
portctl run -- ./server
```

`portctl` automatically:
1. Detects the project name from `package.json`, `Cargo.toml`, `pyproject.toml`, or the directory name.
2. Allocates a unique, deterministic port from the managed registry.
3. Injects the assigned port into the environment variable `$PORT`.
4. Executes your command with child process inheritance.

## 4. Query Assigned Port

To view the assigned port for the current project:

```bash
# Current directory
portctl get

# Or specify by project name
portctl get my-frontend
```

## 5. Expose Locally Running Server to the Internet

When you need to test webhooks (Stripe, GitHub, Shopify) or test on a mobile device, expose your service with Cloudflare Tunnel:

```bash
portctl expose my-app -d preview.example.com
```

This single command:
- Validates the domain name according to RFC 1123.
- Checks socket security guardrails (blocking sensitive internal ports).
- Adds a DNS route in Cloudflare.
- Updates ingress rules in `~/.cloudflared/config.yml`.
- Reloads the background `cloudflared` daemon.

To revoke the tunnel route when finished:

```bash
portctl unexpose my-app
```
