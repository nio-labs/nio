# NioAI 0.3.8

## What's New

- **Kilo Default Provider & Model Fallback Chain**: Defaulted to Kilo Gateway with primary model Mini Code (`kilo::cohere/north-mini-code:free`) and automatic transient error fallback to StepFun (`kilo::stepfun/step-5-preview-free`) and Auto Free (`kilo::kilo-auto/free`).
- **Free Model Ranking Priority**: Promoted Mini Code, StepFun, and Auto Free to top ranking in interactive model picker and catalog selection.
- **NioXS Designated as PoC**: Labeled NioXS as an experimental Proof of Concept provider (`http://127.0.0.1:7460/v1`) with `nioxs::nio-nano`.
- **Custom Provider Endpoint Support**: Ensured CLI `--base-url` flag and saved custom gateway configurations are properly respected during provider resolution.

## Install

```sh
curl -fsSL https://raw.githubusercontent.com/nio-labs/nio/main/install.sh | NIO_VERSION=v0.3.8 bash
```

Or, once published to npm:

```sh
npm install -g @nio-labs/nio-ai@0.3.8
```
