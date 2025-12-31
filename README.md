# Goten: Go Version & Environment Orchestrator

Goten is a Rust-based tool designed to solve the "it works on my machine" problem for Go developers. It intercepts Go commands, detects version mismatches, and automatically re-runs failed commands in isolated, version-compatible containers.

## Core Features

- **Automatic Version Detection**: Scans `go.mod` and `go.sum` to identify the highest Go version required by the project or its dependencies.
- **Deep Isolation Architecture**: 
  - Mounts host source as read-only.
  - Copies project files to a container-internal anonymous volume to prevent host `GOROOT` leakage.
  - Strips host environment variables to ensure zero contamination.
- **Dynamic Version Floor**: Automatically upgrades the container base to `golang:1.25` even if the project specifies an older version, satisfying modern dependency requirements (e.g., Go 1.25+ features).
- **Gemini 2.5 Pro Integration**: Automatically contextualizes build failures and provides AI-generated fixes using Gemini 2.5 Pro when a container build fails.
- **System-Wide Upgrades**: Provides a destructive `update` command to purge legacy Go installations and install the latest stable toolchain globally.
- **Multi-Runtime Fallback**: Supports Docker, Podman, and Kubernetes (`GOTEN_K8S=1`).

## Installation

Build from source and move to your path:

```bash
cd goten
cargo build --release
cp target/release/goten ~/bin/goten
```

## Usage

### 1. Transparent Wrapper
Use `goten` as a prefix to any standard Go command. If the local Go installation fails (e.g., due to version incompatibility), Goten handles the fallback:

```bash
goten test ./...
goten build -o myapp .
```

### 2. System Upgrade
To permanently fix a legacy Go installation on your host:

```bash
goten update
```
*Note: This will purge `/usr/local/go` and `/usr/bin/go` and install Go 1.25.5.*

### 3. Kubernetes Backend
For environments where local Docker is restricted, run Goten in a K8s pod:

```bash
GOTEN_K8S=1 goten build ./...
```

## Configuration

- **API Key**: Place your Gemini API key in `~/.keys/gemini` to enable AI bug analysis.
- **Docker Permissions**: If the user is not in the `docker` group, Goten will automatically attempt a fallback to `podman`.

## Error Analysis Example

If a build fails inside the container, Goten will output:
```text
Isolated build failed. Initiating Gemini 2.5 Pro analysis...

=== Gemini 2.5 Pro Analysis & Fix ===
The error 'undefined: ErrQuotaLimited' indicates a missing definition in internal/agent/hyper/provider.go.
Suggested Fix:
... [Code Diff] ...
=============================
```
