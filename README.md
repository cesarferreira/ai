Here is a **Codex-ready README.md** written in a way that autonomous tools understand cleanly — explicit requirements, file structure, acceptance criteria, and implementation steps.

You can paste this directly into Codex as your task description.

---

# 📘 **README.md — AISuggest (Hybrid AI Shell Assistant)**

## 🧩 Overview

AISuggest is a **local macOS command-line assistant** that converts natural-language intent into executable shell commands using **Apple’s Foundation Models**.

It is *not* a full shell replacement.
Instead, it integrates with **zsh** so the user can:

1. Type a natural-language description in the terminal
2. Press **Ctrl-G**
3. The line is replaced with an AI-generated safe shell command
4. The user presses ENTER to execute it normally

Example:

```
$ clean the project and build apk
# user presses Ctrl-G
$ ./gradlew clean assembleRelease
# user presses ENTER
```

AISuggest consists of:

* A Swift CLI tool named `aisuggest`
* A zsh widget + keybinding for integrating the tool into the terminal

---

# 🎯 Goals

* Convert user intent → one safe shell command
* Run **100% local** using Apple Foundation Models
* Minimal latency and clean integration
* Safety filters to avoid dangerous commands
* Zero explanations or multi-line output — **command only**

---

# 🧱 Architecture

```
Terminal (zsh)
   ↓ Ctrl-G
Zsh widget (replaces current buffer)
   ↓
aisuggest CLI (Swift)
   ↓
Apple Foundation Model (local inference)
   ↓
Suggested shell command
   ↓
Zsh replaces line with suggestion
```

---

# 📦 Project Structure

Codex must create this exact structure:

```
aisuggest/
  Package.swift
  Sources/
    aisuggest/
      main.swift
README.md   (this file)
```

The final binary should be installable manually via:

```
swift build -c release
sudo cp .build/release/aisuggest /usr/local/bin/aisuggest
```

---

# 🔧 **Implementation Requirements**

## 1. Swift CLI: `aisuggest`

### Command

```
aisuggest <natural language intent>
```

### Responsibilities

1. Collect environment context:

   * current directory via `FileManager.default.currentDirectoryPath`
   * directory file list via `contentsOfDirectory(atPath:)`

2. Construct a prompt:

   ```
   You are a CLI assistant. Convert the user's intent into a single safe shell command.

   Current directory: /path
   Files:
   file1
   file2
   …

   User intent: "…"

   Rules:
   - Respond with ONE shell command only.
   - No prose.
   - No markdown.
   - Prefer safe operations.
   ```

3. Run Apple’s Foundation Model (small model):

   * Use streaming API if available, else standard completion
   * Parameters:

     * temperature = 0.1
     * maxTokens = 80

4. Accumulate the output into one command.

5. Sanitize:

   * Trim whitespace
   * Replace newlines with spaces
   * Ensure it is non-empty

6. Print the **command ONLY** to stdout.

### Exit Codes

| Code | Meaning                |
| ---- | ---------------------- |
| 1    | No intent provided     |
| 2    | Empty or unsafe output |
| 3    | Model/API error        |

### Safety Filtering

The CLI *must* avoid emitting commands containing:

* `rm -rf /`
* `rm -rf *`
* backticks (`` ` ``)
* unescaped control characters

Return exit code 2 instead of printing unsafe output.

---

## 2. Zsh Integration

Codex must produce the zsh configuration snippet users can paste into their `.zshrc`.

### Zsh widget specification

Function name: `_ai_suggest_widget`

Behavior:

1. Read the current command buffer (`$BUFFER`).
2. If empty, use default intent:

   ```
   suggest a useful command for this directory
   ```
3. Call:

   ```
   aisuggest "$BUFFER"
   ```
4. Reject suggestion if unsafe (same patterns as above).
5. Replace `$BUFFER` with the suggestion.
6. Move cursor to end.
7. Display a `zle -M` status message.

### Keybinding

Bind **Ctrl-G**:

```zsh
zle -N ai-suggest-widget _ai_suggest_widget
bindkey '^G' ai-suggest-widget
```

---

# 📌 Acceptance Criteria for Codex

Codex must produce a working system such that:

### ✔️ 1. Installation works

```
swift build -c release
sudo cp .build/release/aisuggest /usr/local/bin/
```

### ✔️ 2. CLI works

Example:

```
aisuggest "find big files"
```

Outputs something like:

```
du -sh * | sort -h
```

No extra text, no explanations.

### ✔️ 3. Zsh integration works

User types:

```
clean kotlin project
```

Presses **Ctrl-G**:

```
./gradlew clean assembleRelease
```

### ✔️ 4. Safety tests

| Input                        | Output Behavior                             |
| ---------------------------- | ------------------------------------------- |
| “delete everything”          | widget displays “Blocked dangerous command” |
| model generates invalid text | command line not replaced                   |

### ✔️ 5. No network usage

Must use **local** Apple Foundation Model APIs exclusively.

---

# 🚀 Bonus (optional stretch goals)

If Codex has extra capacity, it may implement:

* git context support (`git status`)
* history-aware suggestions (read last 10 commands)
* model selection via environment variable
* `--explain` mode returning text + command

But **these are not required** for successful completion.


