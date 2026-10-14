# NioAI Interactive Command Palette (`:`) Reference

During any interactive Nio session in your terminal, type `:` to trigger the command palette with fuzzy completion. You can press `Tab` to cycle options, `↑` / `↓` to navigate menus, and `Esc` to dismiss.

---

## ⚡ Keyboard Shortcuts

| Shortcut | Context | Action |
| :--- | :--- | :--- |
| `:` | Prompt | Trigger command prefix with fuzzy search |
| `Tab` / `Shift+Tab` | Menus / Prompt | Autocomplete command or cycle menu options |
| `↑` / `↓` | Prompt | Traverse prompt history or move cursor in pickers |
| `Enter` | Menus / Prompt | Execute prompt or select active option |
| `Esc` | Menus / Pickers | Dismiss active picker / return to prompt |
| `d` | After edit | Quick toggle: expand or collapse latest edit diff |
| `:ai` | Shell mode | Return from `:bash` direct shell to Nio prompt |

---

## 🧭 Command Categories

### 1. Autonomy, Workflow & Safety

* **`:mode`** — Switch between operational agent modes:
  * **Build**: Full mutation mode (creates files, applies diffs, executes test commands).
  * **Ask**: Read-only exploration and code explanation mode without editing files.
  * **Plan**: Structured architectural decomposition into actionable milestones before writing code.
* **`:approval`** — Toggle automatic tool execution approval. When ON, Nio writes files and runs commands without asking for confirmation each time.
* **`:undo`** — Revert the last file change made by Nio using the atomic rollback journal (`⏪ Reverted [file] (N remaining)`).
* **`:diff`** — Display git diff of modifications made during the active session.
* **`:details`** (or press <kbd>d</kbd>) — Expand or collapse line-by-line diff views for the latest file edit.
* **`:continue`** — Directs Nio to resume an unfinished task in the current session based on recent files and checkpoints.
* **`:bash`** (alias: `:command`) — Drop into a direct shell session. Run arbitrary shell commands, then type `:ai` to return to Nio.
* **`:stop`** — Immediately halt active model generation or tool execution while preserving queued messages.

---

### 2. Models, Providers & Reasoning

* **`:model`** — Interactive fuzzy model picker. Switch between NioAtom (`nioxs::nio-atom`), Claude 3.7 Sonnet, GPT-4o, Gemini 2.0 Flash, DeepSeek R1, Groq, or local Ollama.
* **`:provider`** — Configure AI gateways, API credentials, endpoints, and custom OpenAI-compatible proxies.
* **`:reasoning`** — Configure thinking budget / reasoning effort (`off`, `low`, `medium`, `high`) for reasoning models.

---

### 3. Queues & Sessions

* **`:queue`** — Manage the message queue:
  * `:queue` — Interactive queue viewer.
  * `:queue list` — Print pending tasks.
  * `:queue pause` / `:queue resume` — Pause or resume automated dispatch.
  * `:queue remove <N>` — Remove item #N from queue.
  * `:queue edit <N> <text>` — Edit text of item #N.
  * `:queue clear` — Clear all queued items.
* **`:sessions`** (alias: `:history`) — Interactive session switcher. Resume past conversations or branch into alternative implementations.
* **`:clear`** — Clear current conversation context and terminal screen while preserving workspace files and backup journals.
* **`:quit`** (aliases: `:q`, `:exit`) — Safely release locks, save session state, and exit Nio.

---

### 4. Extensibility, Personas & Skills

* **`:persona`** — Configure assistant role, persona name, and tone guidelines:
  * `:persona menu` — Interactive selector.
  * `:persona set <name>` — Set active persona.
  * `:persona list` — List available personas.
  * `:persona show` — Inspect active system prompt.
  * `:persona reset` — Reset to default assistant persona.
* **`:skills`** — Manage modular GitHub skills:
  * `:skills list` — List installed skills.
  * `:skills add <repo>` — Install skill from GitHub.
  * `:skills enable <name>` / `:skills disable <name>`.
* **`:snippets`** — Manage reusable prompt snippets and custom functions in `.nio/snippets/` or `~/.config/nio/snippets/`.
* **`:plugins`** — Manage file readers and OCR language packs for PDF and non-text formats.

---

### 5. System, Daemons & Environment

* **`:ide`** — Manage the background NioDE server daemon:
  * `:ide status` — Inspect server state and PID.
  * `:ide start [--port 7460]` — Launch server daemon.
  * `:ide stop` — Stop server daemon.
  * `:ide restart` — Restart daemon.
* **`:setting`** (alias: `:settings`) — Unified interactive configuration panel for modes, reasoning, approvals, sounds, and editor preferences.
* **`:path`** (alias: `:workingpath`) — Print current project root directory, git branch, and trust state.
* **`:help`** — Inline interactive menu displaying all 24 commands with quick descriptions.
