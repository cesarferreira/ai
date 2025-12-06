# ai

Turn natural-language intent into shell commands using Ollama. Press `Ctrl+G` in your terminal, and your intent gets replaced with an AI-generated command.

## Quick Start

```bash
# Build, install, and set up shell integration
make setup

# Reload your shell
source ~/.zshrc
```

Then type something in your terminal and press `Ctrl+G`!

## Requirements

- [Ollama](https://ollama.ai) running locally
- A model pulled (e.g., `ollama pull llama3.2`)

## Install

```bash
# Build and install binary
make install

# Set up shell integration (zsh/bash/fish auto-detected)
ai init

# Or specify shell explicitly
ai init zsh
ai init bash
ai init fish

# Reload your shell config
source ~/.zshrc
```

Or do it all at once:
```bash
make setup
```

## Usage

**CLI:**
```bash
ai "find large files"
# → prints one command, e.g. du -sh * | sort -h
```

**Shell widget:** Type your intent in the prompt (or leave empty), press `Ctrl+G`, and the buffer is replaced with the AI suggestion.

**List available models:**
```bash
ai models
```

## Configuration

Config is stored at `~/.config/ai/config.yaml`.

```bash
# Show current config
ai config show

# List available models
ai models

# Change Ollama model
ai config set ollama_model mistral

# Change Ollama URL (for remote instances)
ai config set ollama_url http://192.168.1.100:11434
```

**Config options:**
| Key | Default | Description |
|-----|---------|-------------|
| `ollama_model` | `llama3.2` | Ollama model to use |
| `ollama_url` | `http://localhost:11434` | Ollama API endpoint |

**Example config.yaml:**
```yaml
ollama_model: llama3.2
ollama_url: http://localhost:11434
```

## Safety

- Blocks outputs containing `rm -rf /`, `rm -rf *`, backticks, or control characters
- Unsafe or empty results exit with code 2 (CLI) or keep your buffer unchanged
- Only a single command is printed; no markdown or extra text

## How it works

1. Collects context: current directory path and file list
2. Builds a prompt for the LLM with rules to output only a single shell command
3. Sends request to Ollama, cleans the response, applies safety filters, and prints the command

## Troubleshooting

- **"command not found: ai"**: Run `make install` and ensure `/usr/local/bin` is on your `PATH`
- **"model error: connection refused"**: Make sure Ollama is running (`ollama serve`)
- **Ctrl+G not working**: Run `ai init` and reload your shell (`source ~/.zshrc`)
- **Unsafe suggestion blocked**: Refine your intent; suggestions containing dangerous patterns are discarded
