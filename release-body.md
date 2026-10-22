# NioAI 0.3.11 — Terminal Companion to NioXS & Sovereign MicroLMs

## What's New

- 🦈 **Official NioXS Companion Integration**: NioAI is now the official interactive terminal companion and command cockpit for [**NioXS**](https://github.com/nio-labs/nio-xs).
- 🧠 **Default Sovereign Conductor (`nioxs::nio-cuda`)**: Defaulted new sessions to `NioCuda` (Tactical Swarm Conductor & Air-Traffic Controller), enabling sub-10ms local DAG planning within < 160MB resident RAM.
- ⚡ **Built-In MicroLM (µLM) Fleet Catalog**: Injected the complete Nio predator family directly into `:model` and `nio models`:
  - `nioxs::nio-cuda` (Tactical Swarm Conductor & Orchestrator)
  - `nioxs::nio-ray` (Keystroke Predator — Streaming Reflex 24L, < 86 MB RAM)
  - `nioxs::nio-viper` (Tactical Striker — Shell Reflex 8L, < 47 MB RAM)
  - `nioxs::nio-manta` (Conversational & AST Companion — 16L, < 110 MB RAM)
  - `nioxs::nio-nano` (Intent Reflex & MicroLM — ~0.5M, sub-0.1ms)
  - `nioxs::nio-micro` (Contextual Policy Decision Arbiter — 5.0M / cluster)
  - `nioxs::nio-mask` (Byte-Stream Privacy & Secret Redactor — 1.1M, sub-0.2ms)
  - `nioxs::nio-mini` (Predictive Defect & Quality Forecaster — 7.1M, sub-5ms)
- 🔝 **Top Priority Fleet Ranking**: Promoted all native NioXS MicroLM models to Items #1 through #8 on Page 1 of the interactive `:model` picker.
- 🛡️ **Offline Local Catalog Resilience**: The `:model` picker now seamlessly displays the full local MicroLM fleet even if the local NioXS gateway daemon is offline or booting.
- 🔄 **Automatic Cloud Failover**: Preserved instant fallback to free cloud models (`kilo::openrouter/free`, `kilo::cohere/north-mini-code:free`, etc.) when offline or when cloud models are desired.

## Install

```sh
curl -fsSL https://raw.githubusercontent.com/nio-labs/nio/main/install.sh | NIO_VERSION=v0.3.11 bash
```

Or run via Cargo:

```sh
cargo install --git https://github.com/nio-labs/nio.git
```
