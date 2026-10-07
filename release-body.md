# NioAI 0.3.5

## What's New

- **Expanded settings**: Configure provider and persona alongside other options from `:setting`.
- **Command list cleanup**: Mouse support, color theme, and proxy configuration are managed from settings; approval, mode, and reasoning remain standalone commands.
- **Bridge improvements**: Add bridge and assemble workflows, preserve session assessments and resume context, and improve interactive resume.
- **Agent and snippet support**: Detect more coding agents and use `nio-js exec` as the default JavaScript and TypeScript snippet runner, with fallback support.
- **Terminal and streaming fixes**: Improve menu layout and prevent leaked inline tool-call tags from appearing during streaming.

## Install

```sh
curl -fsSL https://raw.githubusercontent.com/nio-labs/nio/main/install.sh | NIO_VERSION=v0.3.5 bash
```

Or, once published to npm:

```sh
npm install -g @nio-labs/nio-ai@0.3.5
```
