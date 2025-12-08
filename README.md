# ai

Turn natural-language intent into shell commands using local LLMs. Features smart context routing that automatically gathers relevant information (git diff, file contents, etc.) before generating commands.

```
$ ai "write a commit message for my changes"

› write a commit message for my changes
Gathering context: status, staged, log
Model: llama3.2 · 12 files + status, staged, log
⠋ Generating... 1.2s
› git commit -m "Add smart routing for context-aware command generation" (2.1s)
Copied to clipboard. Press Cmd+V to paste.
```

## How It Works

```
┌─────────────────────────────────────────────────────────────────┐
│  "write a commit message"                                       │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  Router Model (qwen2.5:0.5b) - Fast, tiny                       │
│  Analyzes intent → decides what context is needed               │
│  Output: { git_diff_staged: true, git_log: true, ... }          │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  Context Gatherers                                              │
│  Runs: git diff --staged, git log --oneline -10, etc.           │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  Main Model (llama3.2, mistral, deepseek, etc.)                 │
│  Generates command with full context                            │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  git commit -m "Add user authentication flow"                   │
└─────────────────────────────────────────────────────────────────┘
```

**Context types the router can request:**
| Context | When Used |
|---------|-----------|
| `git_status` | Commits, understanding repo state |
| `git_diff` | Code reviews, understanding changes |
| `git_diff_staged` | Writing commit messages |
| `git_log` | Commit message style, rebasing |
| `git_branch` | Branch operations, checkouts |
| `file_tree` | Navigation, finding files |
| `read_files` | Reading specific file contents |

## Quick Start

```bash
# 1. Install Ollama (https://ollama.ai)
# 2. Pull the models
ollama pull llama3.2        # Main model (2.0GB)
ollama pull qwen2.5:0.5b    # Router model (394MB) - fast context analyzer

# 3. Build and install
make setup

# 4. Reload your shell
source ~/.zshrc
```

Now try it:
```bash
ai "find all TODO comments in this project"
ai "write a commit message"
ai "show disk usage sorted by size"
```

Or press `Ctrl+G` in your terminal with your intent typed out!

## Recommended Models

### Router Model (fast, for context analysis)
```bash
# Pick ONE - smaller = faster routing
ollama pull qwen2.5:0.5b     # 394MB - recommended, very fast
ollama pull qwen2.5:1.5b     # 986MB - slightly smarter
ollama pull llama3.2:1b      # 1.3GB - alternative
```

### Main Model (for command generation)
```bash
# Pick based on your hardware and needs
ollama pull llama3.2         # 2.0GB - good balance, fast
ollama pull llama3.2:3b      # 2.0GB - same as above
ollama pull mistral          # 4.1GB - great quality
ollama pull deepseek-r1:8b   # 4.9GB - excellent reasoning
ollama pull llama3.3:70b     # 43GB  - best quality (needs good GPU)
```

**Hardware recommendations:**
| RAM | Recommended Main Model |
|-----|------------------------|
| 8GB | llama3.2, qwen2.5:3b |
| 16GB | mistral, deepseek-r1:8b |
| 32GB+ | llama3.3:70b, deepseek-r1:32b |

## Installation

```bash
# Install binary to ~/.cargo/bin
cargo install --path .

# Or use make
make install

# Set up shell integration (zsh/bash/fish auto-detected)
ai init

# Reload your shell config
source ~/.zshrc
```

Or all at once:
```bash
make setup
source ~/.zshrc
```

## Usage

### CLI
```bash
ai "find large files"
# → du -sh * | sort -rh | head -20

ai "write a commit message"
# → (gathers git context first) → git commit -m "..."

ai "show me the last 5 git commits by john"
# → git log --author="john" -5 --oneline
```

### Shell Widget
Type your intent in the terminal and press `Ctrl+G`. The command replaces your input:

```
$ list files modified today<Ctrl+G>
$ find . -mtime 0 -type f
```

### List Models
```bash
ai models
```

## Configuration

Config stored at `~/.config/ai/config.yaml`

```bash
# Show current config
ai config show

# Change main model
ai config set ollama_model mistral

# Change router model
ai config set router_model qwen2.5:1.5b

# Disable smart routing (faster, less context-aware)
ai config set router_enabled false

# Use remote Ollama instance
ai config set ollama_url http://192.168.1.100:11434
```

### Config Options

| Key | Default | Description |
|-----|---------|-------------|
| `ollama_model` | `llama3.2` | Main model for command generation |
| `ollama_url` | `http://localhost:11434` | Ollama API endpoint |
| `router_model` | `qwen2.5:0.5b` | Small model for context analysis |
| `router_enabled` | `true` | Enable smart context routing |

### Example config.yaml
```yaml
ollama_model: mistral
ollama_url: http://localhost:11434
router_model: qwen2.5:0.5b
router_enabled: true
```

## Examples

```bash
# Git operations (auto-gathers git context)
ai "write a commit message"
ai "squash the last 3 commits"
ai "show what changed in the last commit"
ai "create a branch for the login feature"

# File operations
ai "find all files larger than 100MB"
ai "count lines of code in src/"
ai "find and delete all .DS_Store files"
ai "compress all images in this folder"

# System operations
ai "show which process is using port 3000"
ai "list all running docker containers"
ai "how much disk space is left"
ai "show system memory usage"

# Development
ai "run tests and show only failures"
ai "start a local server on port 8080"
ai "install dependencies"
ai "format all python files"
```

## Safety

- Blocks dangerous patterns: `rm -rf /`, `rm -rf *`
- Filters control characters and malformed output
- Commands are copied to clipboard (not auto-executed)
- You always review before pressing Enter

## Requirements

- [Rust](https://rustup.rs/) (for building)
- [Ollama](https://ollama.ai) running locally (or remote)
- Minimum 8GB RAM recommended

## Troubleshooting

| Problem | Solution |
|---------|----------|
| `command not found: ai` | Run `make install`, ensure `~/.cargo/bin` is in PATH |
| `model error: connection refused` | Start Ollama: `ollama serve` |
| `Ctrl+G not working` | Run `ai init` then `source ~/.zshrc` |
| `Router timeout` | Pull router model: `ollama pull qwen2.5:0.5b` |
| `Slow responses` | Use smaller model or disable routing |

### Disable Routing (Faster, Less Smart)
```bash
ai config set router_enabled false
```

This skips the context analysis step - faster but won't automatically gather git info, file contents, etc.

## How It Works (Technical)

1. **Intent Analysis**: Router model (tiny, fast) analyzes your request and returns JSON indicating what context would help
2. **Context Gathering**: System runs relevant commands (`git diff`, `git status`, etc.) based on router output
3. **Command Generation**: Main model receives your intent + gathered context and generates a single shell command
4. **Safety Filtering**: Output is cleaned (strips markdown, backticks) and checked for dangerous patterns
5. **Clipboard**: Command is copied to clipboard for you to paste and review

## License

MIT
