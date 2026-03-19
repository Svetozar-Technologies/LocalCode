# LocalCode CLI

A terminal-based AI coding assistant with autonomous agent capabilities.

## Installation

### From Source

```bash
git clone https://github.com/Svetozar-Technologies/LocalCode.git
cd LocalCode
cargo install --path crates/localcode-cli
```

### Verify

```bash
localcode --version
# localcode 0.2.0
```

## Quick Start

### 1. Set Up a Provider

```bash
# Option A: Ollama (recommended for local/private use)
brew install ollama
ollama serve
ollama pull qwen2.5-coder:7b
localcode config --set default_provider=local
localcode config --set local.server_url=http://127.0.0.1:11434

# Option B: OpenAI
localcode config --set default_provider=openai
localcode config --set openai.api_key=sk-your-key

# Option C: Anthropic
localcode config --set default_provider=anthropic
localcode config --set anthropic.api_key=sk-ant-your-key
```

### 2. Run Your First Task

```bash
cd your-project
localcode agent "show me the project structure and explain it"
```

## Commands

### `localcode` (Interactive REPL)

Starts an interactive session with agent mode enabled by default.

```bash
localcode
agent> find all TODO comments in the codebase
agent> what does the main function do?
agent> add a health check endpoint
```

#### Slash Commands

| Command | Action |
|---------|--------|
| `/help` | Show available commands |
| `/clear` | Clear conversation history |
| `/exit` | Save session and exit |
| `/config` | Show current configuration |
| `/memory` | Show project memory context |
| `/model [name]` | Show or switch model |
| `/commit` | Generate commit message with AI and commit |
| `/agent` or `/a` | Toggle agent mode on/off |

### `localcode agent "<task>"`

Run a single agent task. The agent can use 25+ tools to read/write files, search code, run commands, and interact with git.

```bash
# Code analysis
localcode agent "find all functions that return errors"
localcode agent "count lines of code in src/"
localcode agent "search for how the database connection is configured"

# Code modification
localcode agent "add input validation to the signup form"
localcode agent "create a unit test for the calculate_total function"
localcode agent "refactor the error handling to use custom error types"

# Git operations
localcode agent "show git status and recent commits"
localcode agent "what changed in the last commit?"

# Shell operations
localcode agent "run the test suite and tell me what fails"
```

**Options:**
- `-p, --provider <name>` — Override the default provider (openai, anthropic, local)

### `localcode chat "<message>"`

Send a single chat message (no tools, no agent loop).

```bash
localcode chat "explain what a monad is"
localcode chat "write a regex for email validation" -p openai
```

**Options:**
- `-p, --provider <name>` — Override provider
- `-m, --model <model>` — Override model

### `localcode review`

AI-powered code review of your uncommitted git changes.

```bash
cd my-project
# Make some changes...
localcode review
```

The agent reads `git diff`, analyzes the changes, and provides:
- File-by-file review
- Bug detection
- Style recommendations
- Security concerns
- Actionable improvement suggestions

### `localcode fix "<error>"`

AI-powered error fixing. Paste an error message and the agent will find and fix the issue.

```bash
localcode fix "error[E0308]: expected struct 'Vec', found enum 'Option'"
localcode fix "TypeError: Cannot read property 'map' of undefined"
```

### `localcode init`

Initialize a `LOCALCODE.md` project configuration file.

```bash
cd my-project
localcode init
```

### `localcode config`

Manage configuration.

```bash
localcode config --show                          # Show all settings
localcode config --get openai.api_key            # Get a specific value
localcode config --set default_provider=openai   # Set a value
localcode config --set openai.api_key=sk-...     # Set API key
localcode config --set local.server_url=http://127.0.0.1:11434
```

**Available keys:**
- `default_provider` — "local", "openai", "anthropic"
- `openai.api_key` — OpenAI API key
- `openai.model` — OpenAI model name
- `openai.base_url` — Custom OpenAI-compatible endpoint
- `anthropic.api_key` — Anthropic API key
- `anthropic.model` — Anthropic model name
- `local.server_url` — Local model server URL
- `local.model_path` — Path to GGUF model file (for llama.cpp)

## Configuration

Configuration is stored at `~/.localcode/config.toml`:

```toml
default_provider = "local"

[providers.local]
server_url = "http://127.0.0.1:11434"

[providers.openai]
api_key = "sk-..."
model = "gpt-4o"

[providers.anthropic]
api_key = "sk-ant-..."
model = "claude-sonnet-4-20250514"

[agent]
max_iterations = 15
auto_approve_reads = true
auto_approve_writes = false
auto_approve_commands = false
```

## Model Recommendations

| System Memory | Model | Install Command |
|---------------|-------|----------------|
| 8GB | qwen2.5-coder:7b | `ollama pull qwen2.5-coder:7b` |
| 16GB | qwen2.5-coder:7b | `ollama pull qwen2.5-coder:7b` |
| 32GB+ | qwen2.5-coder:14b | `ollama pull qwen2.5-coder:14b` |

> On 16GB machines, the 14B model may crash on complex multi-step tasks. The 7B model provides reliable tool-calling performance at this memory tier.

## GPU Acceleration

LocalCode uses **Metal** (Apple's GPU API) for inference acceleration:

- **Ollama**: Metal acceleration is automatic on macOS
- **llama.cpp**: Uses `-ngl 99` to offload all layers to Metal GPU
- Works on Apple Silicon (M1/M2/M3/M4) and Intel Macs with Metal support

This is not MPS (PyTorch's Metal backend) — it's direct Metal integration through llama.cpp/Ollama's C++ inference engine, which is more efficient for LLM inference.

## Privacy

- All inference runs on your hardware (when using local provider)
- No data leaves your machine
- No telemetry or analytics
- Configuration stored locally at `~/.localcode/`
- Project memory stored locally at `.localcode/`
