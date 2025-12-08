# term-mate

![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)
![Build Status](https://img.shields.io/badge/build-passing-brightgreen)
![Version](https://img.shields.io/badge/version-0.1.0-blue)

Turn natural-language intent into shell commands using local LLMs. **term-mate** features smart context routing that automatically gathers relevant information (git diff, file contents, etc.) before generating commands.

```bash
$ mate "write a commit message for my changes"

› write a commit message for my changes
Gathering context: status, staged, log
Model: llama3.2 · 12 files + status, staged, log
⠋ Generating... 1.2s
› git commit -m "Add smart routing for context-aware command generation" (2.1s)
Copied to clipboard. Press Cmd+V to paste.
```

## Table of Contents

- [Why term-mate?](#why-term-mate)
- [Quick Start](#quick-start)
- [Installation](#installation)
- [Usage](#usage)
- [Configuration](#configuration)
- [Recommended Models](#recommended-models)
- [Examples](#examples)
- [How It Works](#how-it-works)
- [Troubleshooting](#troubleshooting)
- [Contributing](#contributing)
- [License](#license)

## Why term-mate?

Unlike cloud-based tools or generic CLI helpers, **term-mate**:
1.  **Runs Locally**: Uses Ollama (llama3.2, mistral, etc.) so your code never leaves your machine.
2.  **Smart Context Routing**: A tiny, fast router model analyzes your intent first to decide *what* context is needed (git diffs, file trees, specific file contents) before asking the main model.
3.  **Shell Integration**: Just press `Ctrl+G` in your terminal to replace your text with a command.

## Quick Start

1.  **Install Ollama** ([ollama.ai](https://ollama.ai))
2.  **Pull the models**:
    ```bash
    ollama pull llama3.2        # Main model (2.0GB)
    ollama pull qwen2.5:0.5b    # Router model (394MB) - fast context analyzer
    ```
3.  **Build and install**:
    ```bash
    make setup
    ```
4.  **Reload your shell**:
    ```bash
    source ~/.zshrc  # or ~/.bashrc
    ```

Now try it:
```bash
mate "find all TODO comments in this project"
```

## Installation

### From Source

```bash
# Install binary to ~/.cargo/bin
cargo install --path .

# Or use make
make install
```

### Shell Integration

To enable the `Ctrl+G` widget:

```bash
# Set up shell integration (zsh/bash/fish auto-detected)
mate init

# Reload your shell config
source ~/.zshrc
```

## Usage

### CLI

```bash
mate "find large files"
# → du -sh * | sort -rh | head -20

mate "write a commit message"
# → (gathers git context first) → git commit -m "..."
```

### Shell Widget

Type your intent in the terminal and press `Ctrl+G`. The command replaces your input:

```bash
$ list files modified today<Ctrl+G>
$ find . -mtime 0 -type f
```

### List Models

```bash
mate models
```

## Configuration

Config stored at `~/.config/term-mate/config.yaml`.

```bash
# Show current config
mate config show

# Change main model
mate config set ollama_model mistral

# Change router model
mate config set router_model qwen2.5:1.5b

# Disable smart routing (faster, less context-aware)
mate config set router_enabled false

# Use remote Ollama instance
mate config set ollama_url http://192.168.1.100:11434
```

### Config Options

| Key | Default | Description |
|-----|---------|-------------|
| `ollama_model` | `llama3.2` | Main model for command generation |
| `ollama_url` | `http://localhost:11434` | Ollama API endpoint |
| `router_model` | `qwen2.5:0.5b` | Small model for context analysis |
| `router_enabled` | `true` | Enable smart context routing |

## Recommended Models

### Router Model (fast, for context analysis)
```bash
# Pick ONE - smaller = faster routing
ollama pull qwen2.5:0.5b     # 394MB - recommended, very fast
ollama pull qwen2.5:1.5b     # 986MB - slightly smarter
```

### Main Model (for command generation)
```bash
# Pick based on your hardware and needs
ollama pull llama3.2         # 2.0GB - good balance, fast
ollama pull mistral          # 4.1GB - great quality
ollama pull deepseek-r1:8b   # 4.9GB - excellent reasoning
ollama pull llama3.3:70b     # 43GB  - best quality (needs good GPU)
```

## Examples

### Git Operations
*Auto-gathers git context (diffs, status, logs)*
```bash
mate "write a commit message"
mate "squash the last 3 commits"
mate "show what changed in the last commit"
mate "create a branch for the login feature"
```

### File Operations
```bash
mate "find all files larger than 100MB"
mate "count lines of code in src/"
mate "find and delete all .DS_Store files"
mate "compress all images in this folder"
```

### System Operations
```bash
mate "show which process is using port 3000"
mate "list all running docker containers"
mate "how much disk space is left"
mate "show system memory usage"
```

### Development
```bash
mate "run tests and show only failures"
mate "start a local server on port 8080"
mate "install dependencies"
mate "format all python files"
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

## Troubleshooting

| Problem | Solution |
|---------|----------|
| `command not found: mate` | Run `make install`, ensure `~/.cargo/bin` is in PATH |
| `model error: connection refused` | Start Ollama: `ollama serve` |
| `Ctrl+G not working` | Run `mate init` then `source ~/.zshrc` |
| `Router timeout` | Pull router model: `ollama pull qwen2.5:0.5b` |
| `Slow responses` | Use smaller model or disable routing |

## Contributing

1.  Clone the repo
2.  Install Rust
3.  Build: `cargo build --release`
4.  Run tests: `cargo test`

## License

MIT
