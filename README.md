# 🏯 Goten (悟天)

**Goten** is a high-performance Rust orchestrator designed to provide **Hermetic Go Execution**. It acts as a transparent proxy for `go` commands, automatically detecting version mismatches and re-running failed builds in surgically isolated, version-compatible container environments.

[![Go Version](https://img.shields.io/badge/Go-1.25+-00ADD8?style=flat&logo=go)](https://golang.org)
[![Rust](https://img.shields.io/badge/Rust-2021-orange?style=flat&logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/License-MIT-green)](LICENSE)

---

## 🚀 The Problem
Host environments often lag behind project requirements (e.g., stuck on Go 1.18 while a dependency requires Go 1.25.5). Direct volume mounting in Docker frequently leaks host `GOROOT` metadata, leading to `package cmp is not in GOROOT` errors even inside modern containers.

## 🛡️ The Goten Solution
Goten solves this through **Deep Isolation Architecture**:
1. **Dynamic Floor Detection**: Regex-scans `go.mod` and `go.sum` to identify toolchain requirements. *Note: This is a best-effort heuristic for identifying the required version floor.*
2. **Physical Separation**: Mounts your project as `read-only`, then physically copies it to an anonymous container volume. This breaks all links to host-level Go metadata.
3. **Environment Scrubbing**: Nukes all host environment variables and forces `GOTOOLCHAIN=auto` to allow the containerized Go toolchain to self-upgrade to precise patch versions.
4. **AI Debugging**: If a build fails inside the isolated container, Goten leverages **Gemini 2.5 Pro** to contextualize the error and suggest a code-level fix.

---

## 🛠️ Features
- **Transparent Proxying**: Use `goten` as a drop-in prefix for any `go` command.
- **Hermetic Mode**: Use `--isolated` to skip host checks and jump straight to the container.
- **Multi-Runtime Fallback**: Native support for **Docker**, **Podman**, and **Kubernetes** (`GOTEN_K8S=1`).
- **AI Diagnostics**: Integrated Gemini 2.5 Pro bug analysis via `~/.keys/gemini`.
- **System Maintenance**: `goten update --force` purges legacy system-wide Go installs and replaces them with the latest stable binary.
- **Smart Root Detection**: Automatically finds the `module` boundary, even when run from deep subdirectories.

---

## 📦 Installation

```bash
# Clone and build
git clone git@github.com:elci-group/goten.git
cd goten
cargo build --release

# Install to path
cp target/release/goten ~/bin/goten
```

## ⌨️ Usage

### Standard Workflow
Goten tries the host Go first. If it fails due to versioning, it triggers the isolated container fallback:
```bash
goten test ./...
goten build -o production_binary .
```

### Hermetic Workflow
Force execution in an isolated environment immediately:
```bash
goten --isolated build .
```

### System Maintenance
Purge legacy versions (like 1.18) and install the latest stable Go globally:
```bash
goten update --force
```

### Kubernetes Orchestration
Run builds in an ephemeral K8s Pod for restricted environments:
```bash
GOTEN_K8S=1 goten build .
```

---

## 🤖 AI Bug Contextualization
Goten doesn't just fail; it helps you fix. When a container build fails, Goten automatically triggers:
```text
Isolated build failed. Initiating Gemini 2.5 Pro analysis...

=== Gemini 2.5 Pro Analysis & Fix ===
File: internal/agent/hyper/provider.go
Issue: 'ErrQuotaLimited' is undefined.
Fix: Import 'github.com/example/errors' or define the constant.
=============================
```

## 📄 License
MIT © [ELCI Group](https://github.com/elci-group)
