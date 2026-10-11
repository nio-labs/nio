# NioAI 0.3.7

## What's New

- **Default NioXS Provider and NioNano Model**: Introduced the NioXS preset provider (`http://127.0.0.1:7460/v1`) with `nioxs::nio-nano` as the default model selector, keeping Kilo as a built-in alternative without requiring setup.
- **Provider Connectivity & Doctor Diagnostics**: Added automated provider discovery and connectivity diagnostics for NioXS, ensuring seamless local inspection while honoring saved user models and explicit flags.
- **Interactive Command Listing Fix**: Restored `:persona` and `:provider` commands to interactive help listings and completion menus.
- **Architecture & Roadmap Specifications**: Added comprehensive design specifications and implementation roadmaps for the NioXS non-LLM model ecosystem (`NIOXS_PLAN.md` and `NIOXS_ROADMAP.md`).

## Install

```sh
curl -fsSL https://raw.githubusercontent.com/nio-labs/nio/main/install.sh | NIO_VERSION=v0.3.7 bash
```

Or, once published to npm:

```sh
npm install -g @nio-labs/nio-ai@0.3.7
```
