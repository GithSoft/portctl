# ⚡ `portctl` Master Product Roadmap (2026–2027)
## The Local Network Control Plane & Ingress Engine for Modern Developers & AI Agents

**Target System**: `portctl` (Pure Rust CLI, Local-First, Zero-Daemon)  
**Repository**: [https://github.com/tackelua/portctl](https://github.com/tackelua/portctl)  
**Current Baseline**: v2.0.0  

---

## 1. Executive Summary & Strategic Positioning

### 1.1 The Thesis
Local development environments in 2026 have surpassed the complexity of production infrastructure from a decade ago. A modern engineer or AI coding agent routinely runs 5–15 simultaneous microservices, frontend applications, mock auth providers, local LLMs, and background workers across multiple Git branches and worktrees.

Yet, developer port management remains stuck in the 1990s: hardcoded `PORT=3000` in `.env` files, random `EADDRINUSE` terminal crashes, zombie Node/Python processes holding sockets hostage, and manual tunnel wrangling.

`portctl` v2.0.0 solved the fundamental hurdle: a **pure Rust, zero-daemon, sub-millisecond (<3ms), local-first Single Source of Truth (SSOT)** with partitioned multi-machine sync and automated Cloudflare Tunnel ingress.

### 1.2 The Strategic Wedge (2026–2027 Vision)
To win developer mindshare and become the de-facto standard across engineering teams and autonomous AI coding agents, `portctl` evolves from a *port lookup CLI* into **The Local Network Control Plane**:
1. **Zero-Daemon Local HTTPS & Loopback Routing**: Seamless `https://<app>.localhost` with trusted local certs, eliminating OAuth and cookie headaches without running heavy background daemons.
2. **Universal Ingress Hub**: One-command public and private sharing through Cloudflare Tunnels, Tailscale Funnel/Serve, ngrok, and zero-install SSH reverse tunnels.
3. **AI-Native Operation via Model Context Protocol (MCP)**: Providing AI coding agents (Claude Code, Cursor, Antigravity, OpenClaw) with structured stdio tools to inspect, allocate, run, wait for, and clean up local ports safely.
4. **Declarative Team Workspaces & Worktree Offsets**: Commit `portctl.toml` to Git to ensure zero configuration drift across team members and seamless multi-branch development.

```
       ┌────────────────────────────────────────────────────────┐
       │                Modern AI Coding Agents                 │
       │        (Claude Code, Cursor, Antigravity, OpenClaw)    │
       └───────────────────────────┬────────────────────────────┘
                                   │ JSON-RPC via stdio
                                   ▼
       ┌────────────────────────────────────────────────────────┐
       │                  Official MCP Server                   │
       │         (portctl mcp: Allocate, Probe, Kill, Expose)   │
       └───────────────────────────┬────────────────────────────┘
                                   │
┌──────────────────────────────────┴───────────────────────────────────┐
│                          `portctl` Core Engine                       │
│      (Pure Rust static binary, <3ms, POSIX lock, Zero-Daemon)        │
└──────┬───────────────────────────┬───────────────────────────┬───────┘
       │                           │                           │
       ▼                           ▼                           ▼
┌──────────────┐           ┌──────────────┐            ┌──────────────┐
│  Workspace   │           │  Universal   │            │ Orchestration│
│  & Registry  │           │   Ingress    │            │   & Probing  │
├──────────────┤           ├──────────────┤            ├──────────────┤
│portctl.toml  │           │Cloudflare    │            │portctl wait  │
│Worktree Hash │           │Tailscale     │            │portctl kill  │
│Docker Bridge │           │Local HTTPS   │            │portctl doctor│
│Sync Engine   │           │SSH Ephemeral │            │portctl top   │
└──────────────┘           └──────────────┘            └──────────────┘
```

---

## 2. Competitive Landscape & Adjacent DevTools Analysis

| Solution | Language / Arch | Daemon Required? | Port Registry & Allocator | Local HTTPS (*.localhost / *.test) | Edge Tunnels (Cloudflare/Tailscale) | Health / Readiness Probes | AI Agent MCP Server | Multi-Machine Partitioning |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **`portctl` (Target)** | **Rust (Native)** | **No (Zero-Daemon)** | **Deterministic + Git Worktree** | **Yes (`axum` + `rustls`)** | **Multi-Provider (CF, Tailscale, SSH)** | **Yes (TCP/HTTP probes)** | **Yes (Official stdio)** | **Yes (Local-first partitioned)** |
| **puma-dev** | Go | Yes (launchd/systemd) | No (Arbitrary ports) | Yes (`*.test` via resolver) | No | No | No | No |
| **hotel** | Node.js (Dead) | Yes (Node runtime) | No (Manual config) | Yes (`*.localhost` HTTP) | No | No | No | No |
| **foreman / honcho** | Ruby / Python | No (Procfile runner) | Rigid sequential (5000+) | No | No | No | No | No |
| **overmind** | Go (tmux-based) | Process-bound | No (Static env injection) | No | No | No | No | No |
| **process-compose**| Go | Process-bound (TUI) | No (Static ports in YAML) | No | No | Yes (`readiness_probe`)| No | No |
| **Caddy** | Go | Yes (Daemon/Foreground)| No (Manual reverse proxy)| Yes (Automatic local CA) | No (Requires plugins) | Basic health checks | No | No |
| **LocalCan** | Electron/macOS GUI | Yes (Desktop app) | Basic port mapping | Yes (`*.local` / `.test`) | Proprietary Tunnel | No | Yes (Basic MCP) | No (macOS only) |
| **Tailscale Funnel**| Go | Yes (Tailscale daemon)| No (Network layer only) | Tailnet HTTPS only | Native Tailscale edge | No | No | Tailnet mesh |
| **ngrok** | Go | Yes (Client agent) | No (Tunnel broker only) | No | Proprietary ngrok edge | No | No | Cloud managed |

---

## 3. The 6 Major Pain Points in Modern Local Development

1. **Multi-Project & Git Worktree Collisions**: Developing across multiple Git worktrees for PR reviews collisions on identical base ports.
2. **Startup Race Conditions (Cold-Start Stampedes)**: Frontend boots faster than backend API, triggering network failures and broken initial state.
3. **Local HTTPS & SSL Certificate Nightmares**: SameSite cookies, Passkeys/WebAuthn, and OAuth callbacks require HTTPS even locally.
4. **Team Port Drift & Config Rot**: Inconsistent ports across team members break shared scripts and committed configs.
5. **Docker & Docker Compose Host Collisions**: Host ports (`5432`, `6379`) clash with local native daemon processes.
6. **Autonomous AI Coding Agents Spinning Up Blind Servers**: AI agents create lingering background processes and crash on `EADDRINUSE`.

---

## 4. The 5 Strategic Pillars

### 🏛️ Pillar 1: Core Port Engine & Team Workspaces
- **Declarative `portctl.toml`**: Commit port allocations, environment bindings, and ingress rules directly to Git root.
- **Git Worktree Dynamic Offsetting**: Automatically maps worktrees to collision-free offset ranges using deterministic FNV-1a path hashing:
  $$\text{Worktree Port} = \text{Base Port} + 100 + (\text{FNV-1a}(\text{worktree\_path}) \pmod{50})$$
- **Docker Compose Bridge**: Pre-flight verification of host ports before starting Docker Compose containers.
- **POSIX Advisory File Locking (`fd-lock`)**: Cross-process concurrency safety using a dedicated lockfile (`~/.config/portctl/.lock`). Read paths remain completely lock-free for sub-millisecond execution (<3ms), while write mutations acquire an exclusive advisory lock, avoiding inode replacement pitfalls during atomic renames.

### 🌐 Pillar 2: Universal Local Ingress & Multi-Tunnel Hub
- **Extensible `IngressProvider` Trait**: Plug-and-play architecture for tunneling backends with full lifecycle support (`expose` and `unexpose`).
- **Cloudflare Tunnel (`cloudflare`)**: Production-grade ingress with custom domains, automated CNAME routes, and `portctl unexpose` cleanup.
- **Tailscale Serve & Funnel (`tailscale`)**: Private mesh sharing (`*.ts.net`) and public Funnel ingress.
- **SSH Ephemeral Tunnels (`pinggy` / `localhost.run`)**: Zero-install instant public HTTPS URL via native `ssh -R`.
- **Zero-Daemon Local HTTPS (`*.localhost`)**: Embedded reverse proxy resolving `https://<app>.localhost` with auto-generated root CA trusted in system keychains.
- **Security Guardrails v2**: Sensitive port blacklist (22, 5432, 6379, 8384, 11434, 27017, 2375, 9222) and loopback interface binding verification.

### 🛠️ Pillar 3: Service Orchestration, Health Probing & Observability
- **Zombie Process Hunter (`portctl kill`)**: Graceful `SIGTERM` followed by `SIGKILL` after 3000ms with process tree awareness, instant `EPERM` diagnostics, and sensitive port guardrails.
- **System Diagnostics (`portctl doctor`)**: Audits rogue processes, ghost registrations, dangling ingress rules, and file permissions, with `--fix` automated repair and `--json` output.
- **Readiness Prober (`portctl wait`)**: Blocks script execution until TCP socket or HTTP endpoint (`200 OK`) is ready.
- **Interactive TUI Dashboard (`portctl top`)**: Terminal UI using `ratatui` with live socket statuses, CPU/RSS metrics, and single-key shortcuts (`k` kill, `e` expose, `o` open).

### 🤖 Pillar 4: AI & Automation First (The MCP Era)
- **Official Model Context Protocol (MCP) Server (`portctl mcp`)**: Zero-dependency stdio JSON-RPC server for Claude Code, Cursor, Antigravity, and OpenClaw.
- **Agent Lease Sandboxing (`--ttl`)**: Ephemeral port leases with automatic reclamation after agent tasks finish.
- **Machine-Readable JSON Output**: Universal `--json` support across all subcommands.

### 📦 Pillar 5: Packaging, Ecosystem & Distribution
- **Multi-Architecture Static Binaries**: macOS (arm64, x86_64), Linux (x86_64, arm64 musl), Windows (x86_64).
- **Package Managers**: Homebrew Tap (`tackelua/tap/portctl`), Crates.io (`cargo install portctl-cli`), npm npx wrapper (`@portctl/cli`), AUR, Winget.
- **Shell Auto-Completions**: Dynamic completions for `zsh`, `bash`, and `fish` via `clap_complete`.
- **Official GitHub Action**: `tackelua/setup-portctl@v1`.

---

## 5. Version Milestones & Release Schedule (2026–2027)

| Milestone | Target Window | Theme | Primary Deliverables | Key Dependencies |
| :--- | :--- | :--- | :--- | :--- |
| **v2.1.0** | Q1 2026 | **DX Hardening & Diagnostics** | • `portctl kill <app\|port>` with graceful SIGTERM/SIGKILL & guardrails<br>• `portctl doctor [--fix] [--json]` consistency & permission audit<br>• `portctl unexpose <app\|domain>` automated tunnel revocation<br>• `fd-lock` advisory locking on dedicated `.lock` file<br>• Shell completions for zsh/bash/fish via `clap_complete`<br>• Provider-agnostic `TunnelProvider` trait abstraction<br>• Crates.io official publish (`portctl-cli`) | `fd-lock`, `clap_complete`, `sysinfo`, `libc` |
| **v2.2.0** | Q2 2026 | **Declarative Workspaces & Probing** | • `portctl.toml` project manifest support<br>• Deterministic Git Worktree port offsetting<br>• `portctl wait <app> [--tcp\|--http]` readiness prober<br>• Official Homebrew Tap (`tackelua/homebrew-tap`)<br>• Standalone installer `install.sh` | `toml`, `reqwest` |
| **v2.3.0** | Q3 2026 | **AI Agent Revolution (MCP)** | • `portctl mcp` stdio JSON-RPC server (MCP specification)<br>• Agent lease sandboxing with TTL (`--ttl`)<br>• Machine-readable `--json` output across all commands<br>• GitHub Action `tackelua/setup-portctl@v1`<br>• Cursor & Claude Code integration guide | `serde_json`, `tokio` |
| **v2.4.0** | Q4 2026 | **Universal Multi-Tunnel Ingress** | • Extensible Ingress trait architecture<br>• Tailscale Serve & Funnel provider (`--provider tailscale`)<br>• Zero-install SSH reverse tunnels (Pinggy, localhost.run)<br>• ngrok provider support<br>• Ingress status tracking in registry | `async-trait` |
| **v2.5.0** | Q1 2027 | **Zero-Daemon Local HTTPS** | • Embedded loopback reverse proxy (`portctl proxy`)<br>• RFC 6761 `*.localhost` automated resolution<br>• Automated zero-warning local Root CA generation<br>• System keychain integration (macOS/Linux/Windows)<br>• WebSocket forwarding support | `axum`, `hyper`, `rustls`, `rcgen` |
| **v3.0.0** | Q2–Q3 2027 | **The Local Cloud Control Plane** | • Multi-service DAG orchestration (`portctl up / down`)<br>• Interactive Ratatui Terminal Dashboard (`portctl top`)<br>• Docker & Docker Compose port conflict auto-bridge<br>• Encrypted Git/Cloud Team Registry synchronization<br>• 1-line client SDKs (Node, Python, Go) | `ratatui`, `crossterm`, `bollard` |
