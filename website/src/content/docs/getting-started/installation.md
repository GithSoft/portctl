---
title: Installation
description: Install portctl across macOS, Linux, and Windows.
---

import { Tabs, TabItem } from '@astrojs/starlight/components';

`portctl` is distributed as standalone native binaries with zero external dependencies.

## 1-Line Standalone Installers (Recommended)

<Tabs>
  <TabItem label="macOS & Linux (curl | sh)">
```bash
curl -fsSL https://portctl.pages.dev/install.sh | sh
```

The installer will:
- Auto-detect CPU architecture (Apple Silicon `aarch64` or Intel `x86_64`, Linux `x86_64`).
- Query the latest release tag from GitHub Releases.
- Download the compressed archive and verify its SHA256 cryptographic checksum.
- Install into `/usr/local/bin` (or `$HOME/.local/bin` if non-root).
  </TabItem>

  <TabItem label="Windows (PowerShell)">
```powershell
irm https://portctl.pages.dev/install.ps1 | iex
```

The script will:
- Download the latest Windows MSVC zip archive.
- Verify SHA256 checksum.
- Unpack into `%LOCALAPPDATA%\Programs\portctl`.
- Automatically update both User and active session `$env:PATH`.
  </TabItem>

  <TabItem label="Cargo (Rust Toolchain)">
```bash
cargo install portctl-cli
```
  </TabItem>

  <TabItem label="Homebrew (macOS / Linux)">
```bash
brew install tackelua/tap/portctl
```
  </TabItem>
</Tabs>

---

## Standalone Binary Downloads

If you prefer to download standalone executables directly without running scripts:

| Platform | Target Triple | Direct Executable Download | Checksum |
| :--- | :--- | :--- | :--- |
| **macOS Apple Silicon** | `aarch64-apple-darwin` | [portctl-darwin-arm64](https://github.com/tackelua/portctl/releases/latest/download/portctl-darwin-arm64) | [Checksum](https://github.com/tackelua/portctl/releases/latest/download/portctl-v2.1.0-aarch64-apple-darwin.tar.gz.sha256) |
| **macOS Intel** | `x86_64-apple-darwin` | [portctl-darwin-x86_64](https://github.com/tackelua/portctl/releases/latest/download/portctl-darwin-x86_64) | [Checksum](https://github.com/tackelua/portctl/releases/latest/download/portctl-v2.1.0-x86_64-apple-darwin.tar.gz.sha256) |
| **Linux (glibc)** | `x86_64-unknown-linux-gnu` | [portctl-linux-x86_64](https://github.com/tackelua/portctl/releases/latest/download/portctl-linux-x86_64) | [Checksum](https://github.com/tackelua/portctl/releases/latest/download/portctl-v2.1.0-x86_64-unknown-linux-gnu.tar.gz.sha256) |
| **Windows** | `x86_64-pc-windows-msvc` | [portctl-windows-x86_64.exe](https://github.com/tackelua/portctl/releases/latest/download/portctl-windows-x86_64.exe) | [Checksum](https://github.com/tackelua/portctl/releases/latest/download/portctl-v2.1.0-x86_64-pc-windows-msvc.zip.sha256) |

---

## Shell Completions

Generate auto-completions for your shell:

```bash
# Zsh
portctl completion zsh > ~/.zsh/completions/_portctl

# Bash
portctl completion bash > /etc/bash_completion.d/portctl

# Fish
portctl completion fish > ~/.config/fish/completions/portctl.fish
```
