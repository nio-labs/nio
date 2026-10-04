# Nio Voice & Speech-to-Text Architecture

## 1. Overview & Architectural Decision

Voice-to-text in the Nio ecosystem is designed to run **entirely on the client side** (e.g., `nio-guru`, web interfaces, or desktop clients) rather than inside the headless core engine (`nio`).

The client captures microphone audio, performs local or in-browser speech recognition, and sends **plain transcribed text** to Nio and the underlying LLM.

```
┌───────────────────────────────────────────────────────────────┐
│                      Client (nio-guru)                        │
│                                                               │
│   [ Microphone ] ──> [ Client-side STT ] ──> [ Live Text UI ] │
│                             │                       │         │
│                      (Web Speech API /              │         │
│                       Local Whisper)         User Reviews     │
│                                              & Edits Text     │
└─────────────────────────────────────────────────────┬─────────┘
                                                      │
                                           Plain Transcribed Text
                                                      ▼
┌───────────────────────────────────────────────────────────────┐
│                        Nio Core Engine                        │
│                                                               │
│   Works identically across ALL models (1B to 70B+, local      │
│   Ollama, MiniMax, DeepSeek, Claude, GPT, etc.)               │
└───────────────────────────────────────────────────────────────┘
```

---

## 2. Why Client-Side STT?

### A. Universal Model Compatibility
Most production models and lightweight local models (e.g., Llama 3 8B, Gemma 2 2B/9B, Phi-3, Mistral, MiniMax, DeepSeek) are **text-only** and do not accept multimodal binary audio (`input_audio`).
- **Server/Backend Audio Fallback**: Uploading raw audio causes HTTP `400 Bad Request` or `422 Unprocessable Entity` on all non-multimodal models.
- **Client-Side STT**: Transcribing to text first guarantees that **any model of any size** works seamlessly with voice input.

### B. Lightweight Core Engine
Nio's core value proposition is **blazing speed (<10ms startup)** and an **ultra-light footprint (<20MB RAM)** written in pure Rust.
- Embedding heavy audio frameworks, C++ whisper bindings, or OS microphone permissions directly into the CLI adds runtime complexity and platform fragility.
- Delegating audio capture to client apps preserves Nio's minimal footprint.

### C. Superior User Experience (Live Streaming Preview)
In GUI or web clients:
- **Instant Visual Feedback**: Users see words streaming into the prompt field in real time as they speak.
- **Pre-Submission Editing**: Users can correct terminology, adjust typos, add formatting, or append instructions before submitting to the model.
- **Bandwidth & Latency Savings**: Eliminates the overhead of encoding and uploading large multi-megabyte audio files over the network.

### D. Zero External API Keys & Costs
Relying on cloud Whisper endpoints (Groq, OpenAI) requires dedicated API keys and incurs network/per-minute costs. Client-side STT leverages free built-in OS or browser engines with complete privacy.

---

## 3. Client Implementation Options (for `nio-guru`)

### Option 1: Web Speech API (Recommended for Web & Electron)
Built directly into modern Chromium browsers (Chrome, Edge) and Safari.

- **Zero dependencies**: No external packages or models to download.
- **Real-time streaming**: Provides interim results as the user speaks.
- **Implementation snippet**:
  ```javascript
  const SpeechRecognition = window.SpeechRecognition || window.webkitSpeechRecognition;
  const recognition = new SpeechRecognition();

  recognition.continuous = true;
  recognition.interimResults = true;
  recognition.lang = 'en-US';

  recognition.onresult = (event) => {
    let transcript = '';
    for (let i = event.resultIndex; i < event.results.length; ++i) {
      transcript += event.results[i][0].transcript;
    }
    // Update client prompt input with live text
    promptInput.value = transcript;
  };

  recognition.start();
  ```

### Option 2: In-Browser WebGPU / WASM Whisper (100% Offline & Private)
Using [`@xenova/transformers`](https://github.com/xenova/transformers.js) or `whisper-web`.

- **Completely offline**: Runs inside the browser sandbox using WebGPU or WebAssembly.
- **Zero data leaves the machine**: Perfect for privacy-conscious environments.
- **Model size**: `whisper-tiny` (~40MB) or `whisper-base` (~75MB), cached locally in IndexedDB after the first load.

### Option 3: Native OS Speech Frameworks (Desktop / Tauri)
If `nio-guru` is built with Tauri, Electron, or native frameworks:
- **macOS**: `SFSpeechRecognizer` (`Speech.framework`) with on-device recognition.
- **Windows**: Windows Media Speech Recognition / Windows.Media.SpeechRecognition.
- **Linux**: Local `whisper.cpp` sidecar.

---

## 4. Contract Between Client (`nio-guru`) and Nio Core

The client application communicates with `nio` via standard input, CLI args, or IPC:

1. **Standard Prompt Mode**:
   ```sh
   nio "Transcribed user voice prompt text"
   ```
2. **Interactive Session Streaming**:
   Client writes the finalized prompt text into `nio`'s standard input or message queue.
3. **Audio File Attachments vs. Voice Input**:
   - **Voice dictation**: Always converted to text on the client before sending.
   - **Audio file inspection** (e.g., debugging a `.wav` file or meeting recording): Handled via file attachments (`-a audio.wav`) for models that explicitly support multimodal audio.

---

## 5. Recommended Client UX Flow

1. **Activation**:
   - Clickable Microphone icon next to the prompt input.
   - Optional global hotkey (e.g., Hold `Space` or `Cmd+Shift+V` for Push-to-Talk).
2. **Visual State**:
   - Pulsing waveform or recording dot indicating active microphone capture.
3. **Interim Transcription**:
   - Dimmed or italicized text showing streaming words as they are recognized.
4. **Finalization**:
   - User stops speaking or releases hotkey.
   - Text turns into editable regular text in the input box.
5. **Submission**:
   - User either reviews and presses `Enter`, or auto-submits if hands-free mode is toggled on.
