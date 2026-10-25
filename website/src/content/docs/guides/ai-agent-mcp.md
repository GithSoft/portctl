---
title: AI Coding Agents & MCP
description: Integrate portctl stdio JSON-RPC server with Claude Code, Cursor, Antigravity, and OpenClaw.
---

Modern AI coding agents (Claude Code, Cursor, Antigravity, OpenClaw) frequently launch development servers, background workers, and mock databases in sub-processes. Without deterministic port management, agents collide on default ports (`3000`, `8080`) or leave orphaned listening sockets behind.

`portctl` includes native support for the **Model Context Protocol (MCP)** via stdio JSON-RPC.

## Claude Desktop Configuration

Add `portctl` to your Claude Desktop configuration (`~/Library/Application Support/Claude/claude_desktop_config.json` on macOS or `%APPDATA%\Claude\claude_desktop_config.json` on Windows):

```json
{
  "mcpServers": {
    "portctl": {
      "command": "portctl",
      "args": ["mcp"]
    }
  }
}
```

## Cursor IDE Configuration

Add to `.cursor/mcp.json` in your workspace root:

```json
{
  "mcpServers": {
    "portctl": {
      "command": "portctl",
      "args": ["mcp"]
    }
  }
}
```

---

## MCP Tools Exposed

When running in MCP mode (`portctl mcp`), the agent gains access to the following structured tools:

1. **`portctl_list`**: Inspect all registered services and active listening sockets.
2. **`portctl_get`**: Retrieve the assigned port for the current workspace directory or project name.
3. **`portctl_check`**: Real-time socket binding verification and PID diagnostic.
4. **`portctl_kill`**: Gracefully terminate zombie processes occupying required ports.
5. **`portctl_expose`**: Wire a public Cloudflare Tunnel ingress route for interactive user testing.
6. **`portctl_doctor`**: Audit lockfiles, dangling rules, and permissions with automatic repair.

## Machine-Readable Output

All `portctl` CLI commands support `--json` for direct parsing by scripts and agent pipelines:

```bash
portctl list --json
portctl doctor --json
portctl check 3000 --json
```
