# NioAI 0.3.4

## What's New

- **Custom Persona & Built-In Presets**: Give Nio a distinctive persona, custom assistant name, gender & pronouns, and multi-rule behavioral instructions (`:persona`).
- **10 Built-In Personality Presets**: Choose instantly from diverse archetypes:
  - `bff` (Bestie / Alex) — loyal, supportive ride-or-die coding companion with fun banter
  - `partner` (Devoted Partner / Riley) — caring companion who encourages, rest reminders & hydration
  - `ex` (Sassy Ex / Taylor) — witty, skeptical perfectionist pushing for bulletproof code
  - `architect` (Senior Architect) — SOLID design patterns, scalability, and trade-off analysis
  - `rustacean` (Staff Rustacean / Ferris) — zero-cost abstractions, lifetimes, and safety
  - `minimalist` (Code Minimalist / Unix) — pure high-density code and diffs with zero pleasantries
  - `security` (Security Auditor / Sentinel) — defense-in-depth and vulnerability vetting
  - `devops` (DevOps SRE / Ops) — observability, resilience, and containerization
  - `junior` (Junior Enthusiast / Pip) — curious learner who explains complex concepts simply
  - `tutor` (Socratic Tutor / Mentor) — guided educator prompting critical thinking
- **Interactive Persona Picker**: Browse and switch personas interactively in the terminal (`nio persona`) and full-screen TUI (`:persona`).
- **Gender & Pronoun Alignment**: Explicit support for `--gender female|male|non-binary|reset` and `:persona gender <female|male|non-binary|reset>` to align self-identity and character pronouns.
- **Session Header Reflection**: Persona name, active preset, gender, and rule counts are immediately rendered in the session startup header.

## Install

```sh
curl -fsSL https://raw.githubusercontent.com/nio-labs/nio/main/install.sh | NIO_VERSION=v0.3.4 bash
```

Or, once published to npm:

```sh
npm install -g @nio-labs/nio-ai@0.3.4
```
