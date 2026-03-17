# LocalCode

**Privacy-first, AI-powered code editor that runs entirely on your machine.**

LocalCode is a desktop IDE with built-in AI coding assistance powered by local LLMs via [llama.cpp](https://github.com/ggerganov/llama.cpp). No cloud. No telemetry. No data leaves your device.

Built for developers and enterprises who need AI-assisted coding without compromising on privacy or security.

## Features

### Editor
- **Monaco Editor** — Same editing engine as VS Code
- Multi-tab support with syntax highlighting for 25+ languages
- Bracket pair colorization, minimap, font ligatures
- Keyboard shortcuts (Cmd+S save, Cmd+P quick open)

### AI Assistant
- **Chat** — Ask questions about your code, get explanations, generate code
- **Inline Completion** — Ghost-text code suggestions as you type (Tab to accept)
- **Agent Mode** — Autonomous AI that can read/write files, run terminal commands, search your codebase, and execute multi-step tasks
- Context-aware — automatically includes your current file as context

### IDE Features
- **File Explorer** — Tree view with lazy-loading, file type icons, git status badges
- **Integrated Terminal** — Full PTY terminal with color support and resize
- **Search** — File name search and content search across entire project
- **Git Integration** — Branch display, file status indicators, diff, commit log

### Privacy & Performance
- **100% Local** — All AI inference runs on your hardware via llama.cpp
- **Metal GPU Acceleration** — Optimized for Apple Silicon (M1/M2/M3/M4)
- **Offline Capable** — Works without internet after model download
- **No Telemetry** — Zero data collection, no analytics, no phone-home
- **Open Source** — MIT licensed, fully auditable

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop Framework | [Tauri v2](https://tauri.app/) (Rust) |
| Frontend | React + TypeScript |
| Code Editor | [Monaco Editor](https://microsoft.github.io/monaco-editor/) |
| Terminal | [xterm.js](https://xtermjs.org/) + portable-pty |
| AI Inference | [llama.cpp](https://github.com/ggerganov/llama.cpp) (llama-server) |
| Git | libgit2 |
| Search | ignore + walkdir |

## Getting Started

### Prerequisites

- **macOS** 12+ (Apple Silicon or Intel)
- **Rust** 1.77+ — `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **Node.js** 18+ — `brew install node`
- **llama.cpp** — `brew install llama.cpp`

### Install from DMG

Download the latest `.dmg` from [Releases](https://github.com/Svetozar-Technologies/LocalCode/releases), open it, and drag LocalCode to Applications.

### Build from Source

```bash
# Clone the repo
git clone https://github.com/Svetozar-Technologies/LocalCode.git
cd LocalCode/frontend

# Install frontend dependencies
npm install

# Build the app (creates .app and .dmg)
cargo tauri build

# Or run in development mode
cargo tauri dev
```

## Using AI Features

LocalCode uses llama.cpp's server mode for AI inference. You need a GGUF model file.

### 1. Download a Model

Recommended coding models:

| Model | Size | Best For |
|-------|------|----------|
| [DeepSeek-Coder-V2-Lite](https://huggingface.co/TheBloke/deepseek-coder-6.7B-instruct-GGUF) | ~4GB | General coding, 8GB RAM |
| [Qwen2.5-Coder-7B](https://huggingface.co/Qwen/Qwen2.5-Coder-7B-Instruct-GGUF) | ~5GB | Strong coding, 16GB RAM |
| [CodeLlama-13B](https://huggingface.co/TheBloke/CodeLlama-13B-Instruct-GGUF) | ~8GB | Advanced coding, 16GB+ RAM |

### 2. Load the Model

1. Open LocalCode
2. Click the AI panel (globe icon in activity bar)
3. Click the load button next to "No model loaded"
4. Select your `.gguf` model file
5. Wait for the model to load (status dot turns green)

### 3. Start Coding with AI

- **Chat:** Type questions in the AI panel
- **Agent Mode:** Toggle "Agent Mode" checkbox, then describe a task (e.g., "Add error handling to the login function")
- **Inline Completion:** Just start typing — suggestions appear as ghost text

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Cmd+S` | Save file |
| `Cmd+B` | Toggle sidebar |
| `Cmd+`` ` | Toggle terminal |
| `Cmd+I` | Open AI chat |
| `Cmd+P` | Quick open file |
| `Cmd+Shift+F` | Search in files |

## For Enterprises

LocalCode is designed for organizations that cannot use cloud-based AI coding tools due to compliance requirements:

- **HIPAA** — Healthcare organizations handling patient data
- **ITAR/EAR** — Defense contractors with export-controlled code
- **SOC2** — Companies with strict data residency requirements
- **Air-gapped environments** — Networks without internet access

**Enterprise features** (coming soon):
- Admin dashboard for model and policy management
- Audit logging for compliance
- SSO/LDAP integration
- Private model registry
- Air-gapped deployment packages

Contact us for enterprise licensing: [Svetozar Technologies](https://svetozar-technologies.github.io/)

## Architecture

```
┌─────────────────────────────────────────────────┐
│                   LocalCode App                  │
│  ┌──────────┐  ┌──────────┐  ┌───────────────┐ │
│  │  Monaco   │  │ Terminal  │  │   AI Chat     │ │
│  │  Editor   │  │ (xterm)  │  │   + Agent     │ │
│  └────┬─────┘  └────┬─────┘  └──────┬────────┘ │
│       │              │               │           │
│  ┌────┴──────────────┴───────────────┴────────┐ │
│  │           Tauri IPC Bridge                  │ │
│  └────┬──────────────┬───────────────┬────────┘ │
│       │              │               │           │
│  ┌────┴─────┐  ┌────┴─────┐  ┌─────┴────────┐ │
│  │ File Sys  │  │   PTY    │  │ llama-server │ │
│  │ + Git     │  │ Process  │  │ (llama.cpp)  │ │
│  │ (Rust)    │  │ (Rust)   │  │ Metal GPU    │ │
│  └──────────┘  └──────────┘  └──────────────┘ │
└─────────────────────────────────────────────────┘
```

## Contributing

We welcome contributions! LocalCode is MIT licensed.

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

MIT License — see [LICENSE](LICENSE) for details.

## About

Built by [Svetozar Technologies](https://svetozar-technologies.github.io/) — Democratizing AI while protecting privacy.

*AI Should Be Free. AI Should Be Private. AI Should Be Yours.*
