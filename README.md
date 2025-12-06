AISuggest turns natural-language intent into a single safe shell command using Apple's on-device Foundation Model Small. It integrates with zsh so pressing Ctrl-G replaces the current line with an AI-generated suggestion; the user still executes it manually.

## Install
- Build: `swift build -c release`
- Install binary: `sudo cp .build/release/aisuggest /usr/local/bin/`
- Add the contents of `zsh_integration_snippet.txt` to your `~/.zshrc`, then restart your shell.

## Usage
- CLI: `aisuggest "find large files"` → prints one command, e.g. `du -sh * | sort -h`
- zsh: type intent in the prompt (or leave empty), press Ctrl-G, the buffer is replaced with the suggestion and the cursor moves to the end.

## Safety
- The CLI and widget block outputs containing `rm -rf /`, `rm -rf *`, backticks, or control characters. Unsafe or empty results exit with code 2 (CLI) or keep your buffer unchanged (widget shows “Blocked dangerous command”).
- Only a single command is printed; no markdown or extra text.

## How it works
- Collects context: current directory path and a file list.
- Builds a deterministic prompt for the Foundation Model Small (temperature 0.1, maxTokens 80). If the Apple AI framework is unavailable, a conservative heuristic generator is used as a local fallback.
- Streams/collects the model output, flattens newlines, trims whitespace, applies safety filters, and prints the command.

## Troubleshooting
- “command not found: aisuggest”: reinstall to `/usr/local/bin/` and ensure it is on `PATH`.
- Unsafe suggestion blocked: refine the intent; suggestions containing dangerous patterns are discarded.
- Model errors (exit 3): ensure macOS 15+ with Apple AI frameworks installed; the fallback heuristic keeps the tool responsive but may be less accurate.
