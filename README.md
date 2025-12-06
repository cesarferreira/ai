AISuggest turns natural-language intent into a single safe shell command using Ollama. It integrates with zsh so pressing Ctrl-G replaces the current line with an AI-generated suggestion; the user still executes it manually.

## Install

```bash
# Build
cargo build --release

# Install binary
sudo cp target/release/aisuggest /usr/local/bin/

# Or use make
make install
```

Add the contents of `zsh_integration_snippet.txt` to your `~/.zshrc`, then restart your shell.

## Requirements

- [Ollama](https://ollama.ai) running locally
- A model pulled (e.g., `ollama pull llama3.2`)

## Usage

**CLI:**
```bash
aisuggest "find large files"
# → prints one command, e.g. du -sh * | sort -h
```

**zsh widget:** Type intent in the prompt (or leave empty), press Ctrl-G, the buffer is replaced with the suggestion.

## Configuration

Config is stored at `~/.config/aisuggest/config.json`.

```bash
# Show current config
aisuggest config show

# Change Ollama model
aisuggest config set ollama_model mistral

# Change Ollama URL (for remote instances)
aisuggest config set ollama_url http://192.168.1.100:11434
```

**Config options:**
| Key | Default | Description |
|-----|---------|-------------|
| `ollama_model` | `llama3.2` | Ollama model to use |
| `ollama_url` | `http://localhost:11434` | Ollama API endpoint |

## Safety

- Blocks outputs containing `rm -rf /`, `rm -rf *`, backticks, or control characters
- Unsafe or empty results exit with code 2 (CLI) or keep your buffer unchanged (widget shows "Blocked dangerous command")
- Only a single command is printed; no markdown or extra text

## How it works

1. Collects context: current directory path and file list
2. Builds a prompt for the LLM with rules to output only a single shell command
3. Sends request to Ollama, cleans the response, applies safety filters, and prints the command

## Troubleshooting

- **"command not found: aisuggest"**: Reinstall to `/usr/local/bin/` and ensure it's on `PATH`
- **"model error: connection refused"**: Make sure Ollama is running (`ollama serve`)
- **Unsafe suggestion blocked**: Refine your intent; suggestions containing dangerous patterns are discarded
