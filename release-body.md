# NioAI 0.3.6

## What's New

- **Expanded Attachment Limits**: Increased maximum file attachment size limit for text and source code files from 128 KiB to 512 KiB, enabling inspection of larger project documents and datasets.
- **Semantic CLI & TUI Styling**: Added refined color-coding and metadata dimming across terminal output, including distinct styling for user prompts (`You:`) and assistant responses (`nio:`).
- **Interactive Prompt Colorization**: Improved syntax contrast and visual separation with colorized interactive prompts and top padding.
- **Polished Startup Panel**: Enhanced startup banner rendering and persistent persona guidance in interactive sessions.
- **Process Isolation Fix**: Detached child processes using `setsid()` to prevent interactive terminal hangs when executing commands such as `sudo`.

## Install

```sh
curl -fsSL https://raw.githubusercontent.com/nio-labs/nio/main/install.sh | NIO_VERSION=v0.3.6 bash
```

Or, once published to npm:

```sh
npm install -g @nio-labs/nio-ai@0.3.6
```
