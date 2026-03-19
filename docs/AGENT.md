# LocalCode Agent — Technical Documentation

The LocalCode Agent is an autonomous AI coding assistant that can plan, investigate, and execute multi-step coding tasks. It operates through an iterative tool-calling loop, using 25+ built-in tools to interact with files, git, shell commands, and code search.

## How It Works

```
User Task ──> Agent Engine ──> LLM Provider ──> Tool Call ──> Execute ──> Feed Result Back
                   │                                                          │
                   └──────────────────── Iterate (up to 30 times) ───────────┘
                   │
                   └──> Final Text Response ──> User
```

### Execution Flow

1. User provides a task (e.g., "add error handling to the login function")
2. Agent builds a system prompt with:
   - Thinking framework (UNDERSTAND → INVESTIGATE → PLAN → EXECUTE → VERIFY)
   - Tool definitions (JSON schemas for all 25+ tools)
   - Project memory context (framework, language, conventions)
   - Project path and current file (if any)
3. Sends the task + system prompt to the LLM provider
4. LLM returns either:
   - **Text** — task is complete, return to user
   - **Tool call(s)** — agent executes the tool, feeds result back, loops
5. Repeats up to `max_iterations` (default: 30)

### Tool Calling Modes

| Mode | How It Works | When Used |
|------|-------------|-----------|
| **Native** | LLM returns structured `tool_calls` field | OpenAI, Anthropic |
| **Content Fallback** | Agent parses JSON tool calls from response text | Ollama, local models |
| **XML** | Agent parses `<tool>` tags from response text | Legacy fallback |

The content fallback parser handles:
- Single JSON objects: `{"name": "read_file", "arguments": {"path": "src/main.rs"}}`
- JSON arrays: `[{"name": "git_status", "arguments": {}}, {"name": "git_log", "arguments": {}}]`
- Multiple JSON objects separated by whitespace
- JSON inside markdown code blocks (` ```json ... ``` `)

---

## Built-in Tools

### File Operations

| Tool | Description | Parameters |
|------|-------------|------------|
| `read_file` | Read file contents (up to 8KB) | `path` |
| `write_file` | Create or overwrite a file | `path`, `content` |
| `edit_file` | Find and replace text in a file | `path`, `old_text`, `new_text` |
| `create_file` | Create a new file (fails if exists) | `path`, `content` |
| `delete_file` | Delete a file | `path` |
| `list_dir` | List directory contents | `path` |
| `open_in_editor` | Open file in the desktop editor UI | `path` |

All file paths are sandboxed to the project root directory. Absolute paths are resolved relative to the project.

### Search Tools

| Tool | Description | Parameters |
|------|-------------|------------|
| `search_files` | Find files by name pattern | `pattern`, `path` (optional) |
| `search_content` | Search file contents for text | `query`, `path` (optional) |
| `glob` | Glob pattern matching | `pattern` |
| `codebase_search` | Semantic search with RAG | `query`, `top_k` (default: 8) |
| `grep_search` | Regex search with file type filtering | `pattern`, `path`, `include` |
| `find_files` | Find files/dirs by name pattern | `path`, `name`, `type` |

### Git Tools

| Tool | Description | Parameters |
|------|-------------|------------|
| `git_status` | Show modified, untracked, staged files | (none) |
| `git_diff` | Show working tree changes | (none) |
| `git_commit` | Stage all changes and commit | `message` |
| `git_log` | Show commit history | `count` (default: 10) |

### Command Tools

| Tool | Description | Parameters |
|------|-------------|------------|
| `run_command` | Execute shell command in project dir | `command` |
| `grep_search` | Regex content search | `pattern`, `path`, `include` |
| `find_files` | Find files by name | `path`, `name`, `type` |
| `http_request` | Make HTTP requests (GET/POST) | `url`, `method`, `body` |
| `sed_replace` | Find/replace with regex | `file`, `pattern`, `replacement` |
| `count_lines` | Count lines in files | `path` |

### Memory Tools

| Tool | Description | Parameters |
|------|-------------|------------|
| `codebase_search` | Semantic code search with auto-indexing | `query`, `top_k` |
| `update_memory` | Save a fact to project memory | `fact` |

### Multi-Agent Tools

| Tool | Description | Parameters |
|------|-------------|------------|
| `dispatch_subagent` | Spawn a specialized sub-agent | `role`, `task` |

---

## Multi-Agent System

The agent can dispatch specialized sub-agents for complex tasks:

### Agent Roles

| Role | System Prompt Focus | Best For |
|------|-------------------|----------|
| **Searcher** | Code search, file discovery, structured summaries | Finding relevant code across the codebase |
| **Coder** | Implementation, style matching, imports | Writing or modifying code |
| **Reviewer** | Bug detection, style issues, security review | Reviewing changes for quality |

Sub-agents run with their own tool registry and a max of 15 iterations.

---

## Memory System

### Project Memory (`{project}/.localcode/memory/project.json`)

Auto-discovered on first run:
- **Framework** — React, Django, Express, Rails, etc.
- **Language** — Rust, TypeScript, Python, Go, etc.
- **Build system** — Cargo, npm, pip, Make, etc.
- **Build/test/lint commands** — detected from config files
- **File tree summary** — key directories and their purpose
- **Learned conventions** — patterns observed across sessions
- **Session history** — tasks completed, files modified

### Global Memory (`~/.localcode/memory/global.json`)

Shared across all projects.

### Memory in Prompts

The agent's system prompt includes the memory context:
```
# Context
Framework: React + TypeScript
Build: npm run build
Test: npm test
Key directories: src/components/, src/api/, tests/
Conventions: Use functional components, prefer hooks
```

---

## RAG Pipeline (Semantic Code Search)

The `codebase_search` tool uses a hybrid retrieval pipeline:

### Indexing

1. **Walk** — traverse project files, respect `.gitignore`
2. **Chunk** — split into 80-line semantic chunks
3. **Enrich** — extract function signatures, prepend as metadata
4. **Embed** — generate 256-dimension vectors using bigram hashing
5. **Store** — persist to `.localcode/index.json` with file hashes

### Search

1. **Query Embed** — embed the search query using the same algorithm
2. **Cosine Similarity** — find semantically similar chunks (40% weight)
3. **BM25 Scoring** — keyword relevance via TF-IDF (60% weight)
4. **Hybrid Rank** — combine both scores, return top-k results

### Auto-Indexing

- Index is built on first search if none exists
- Stale index (>5 minutes) triggers re-indexing
- Incremental updates using file hash comparison

---

## System Prompt

The agent uses a comprehensive reasoning framework:

```
# Identity & Role
You are LocalCode Agent — an expert software engineer embedded in a code editor.

# Thinking Framework
1. UNDERSTAND — Read the request, identify files/systems involved
2. INVESTIGATE — Use search/read tools to understand current state
3. PLAN — Decide which files to modify and in what order
4. EXECUTE — Make changes using write_file/edit_file
5. VERIFY — Run tests/build to confirm changes work

# Tool Usage Strategy
- START with codebase_search or search_content to find relevant code
- Use read_file to understand context before using edit_file
- Prefer edit_file over write_file for existing files
- Use run_command for build, test, lint, git operations

# Code Quality Rules
- Match existing code style
- Handle errors properly
- Don't leave TODO/FIXME comments

# Error Recovery
- If a tool returns an error, read the error and adapt
- If a file doesn't exist, search for it
- Don't repeat failing actions — try a different approach
```

---

## Configuration

### Agent Settings (in `~/.localcode/config.toml`)

```toml
[agent]
max_iterations = 15          # Max tool-calling iterations
auto_approve_reads = true    # Auto-approve file read operations
auto_approve_writes = false  # Require approval for file writes
auto_approve_commands = false # Require approval for shell commands
```

### Provider Selection

The agent uses whichever LLM provider is configured:

```bash
# Use Ollama (local)
localcode config --set default_provider=local

# Use OpenAI
localcode config --set default_provider=openai
localcode config --set openai.api_key=sk-...

# Use Anthropic
localcode config --set default_provider=anthropic
localcode config --set anthropic.api_key=sk-ant-...
```

---

## Usage Examples

### CLI

```bash
# Simple tasks
localcode agent "list all files in the src directory"
localcode agent "show git status and recent commits"

# Code analysis
localcode agent "find all TODO comments in the codebase"
localcode agent "count total lines of Rust code in crates/"
localcode agent "search for how authentication works"

# Code modification
localcode agent "add error handling to the login function"
localcode agent "create a Python script that prints hello world and run it"
localcode agent "refactor the config module to use builder pattern"

# Code review
localcode review

# Error fixing
localcode fix "cargo build error: expected struct found enum"
```

### Desktop App

1. Open the AI Chat panel (Cmd+I or globe icon)
2. Toggle "Agent Mode" checkbox
3. Type your task
4. Watch the agent work in real-time (tool calls are displayed)

### Interactive REPL

```bash
localcode
# Starts in agent mode by default
agent> find all functions that return Result types
# Agent searches, reads files, summarizes
agent> /a
# Toggle to chat mode for simple questions
```
