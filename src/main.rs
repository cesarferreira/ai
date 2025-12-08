use crossterm::{
    cursor,
    style::{Color, Print, SetForegroundColor, ResetColor},
    terminal::{self, ClearType},
    ExecutableCommand,
};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;

// ============================================================================
// Safety Filter
// ============================================================================

fn is_safe(command: &str) -> bool {
    let lowered = command.to_lowercase();
    if lowered.contains("rm -rf /") {
        return false;
    }
    if lowered.contains("rm -rf *") {
        return false;
    }
    // Block control characters (except tab and newline)
    if command.chars().any(|c| (c as u32) < 0x20 && c != '\t' && c != '\n') {
        return false;
    }
    true
}

// ============================================================================
// Prompt Builder
// ============================================================================

fn build_prompt(intent: &str, working_directory: &str, files: &[String]) -> String {
    let file_list = files.join("\n");
    format!(
        r#"You are a CLI assistant. Convert the user's intent into a single shell command.

Current directory: {}
Files:
{}

User intent: "{}"

STRICT RULES:
- Output ONLY the command itself, nothing else
- NO markdown, NO backticks, NO code blocks
- NO explanations, NO comments, NO alternatives
- ONE single line command only
- Do NOT wrap in quotes or backticks"#,
        working_directory, file_list, intent
    )
}

// ============================================================================
// File Context Collector
// ============================================================================

fn collect_files() -> Vec<String> {
    let current_dir = env::current_dir().unwrap_or_default();
    let mut files: Vec<String> = fs::read_dir(&current_dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter_map(|e| e.file_name().into_string().ok())
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

// ============================================================================
// Configuration
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Backend {
    Ollama,
}

impl Default for Backend {
    fn default() -> Self {
        Backend::Ollama
    }
}

impl std::fmt::Display for Backend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Backend::Ollama => write!(f, "ollama"),
        }
    }
}

impl std::str::FromStr for Backend {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ollama" => Ok(Backend::Ollama),
            _ => Err(format!("Unknown backend: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    #[serde(default)]
    backend: Backend,
    #[serde(default = "default_ollama_model")]
    ollama_model: String,
    #[serde(default = "default_ollama_url")]
    ollama_url: String,
}

fn default_ollama_model() -> String {
    "llama3.2".to_string()
}

fn default_ollama_url() -> String {
    "http://localhost:11434".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Config {
            backend: Backend::default(),
            ollama_model: default_ollama_model(),
            ollama_url: default_ollama_url(),
        }
    }
}

impl Config {
    fn config_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_default()
            .join(".config")
            .join("ai")
    }

    fn config_path() -> PathBuf {
        Self::config_dir().join("config.yaml")
    }

    fn legacy_json_path() -> PathBuf {
        Self::config_dir().join("config.json")
    }

    fn load() -> Self {
        let yaml_path = Self::config_path();
        let json_path = Self::legacy_json_path();

        // Try YAML first
        if yaml_path.exists() {
            if let Ok(content) = fs::read_to_string(&yaml_path) {
                if let Ok(config) = serde_yaml::from_str(&content) {
                    return config;
                }
            }
        }

        // Fall back to legacy JSON
        if json_path.exists() {
            if let Ok(content) = fs::read_to_string(&json_path) {
                if let Ok(config) = serde_json::from_str::<Config>(&content) {
                    // Migrate to YAML
                    let _ = config.save();
                    let _ = fs::remove_file(&json_path);
                    return config;
                }
            }
        }

        Config::default()
    }

    fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_yaml::to_string(self)?;
        fs::write(&path, content)?;
        Ok(())
    }
}

// ============================================================================
// Ollama Client
// ============================================================================

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
    #[serde(default)]
    done: bool,
}

#[derive(Deserialize)]
struct OllamaModel {
    name: String,
    size: u64,
}

#[derive(Deserialize)]
struct OllamaModelsResponse {
    models: Vec<OllamaModel>,
}

fn list_ollama_models(config: &Config) -> Result<Vec<OllamaModel>, Box<dyn std::error::Error>> {
    let url = format!("{}/api/tags", config.ollama_url);
    let client = reqwest::blocking::Client::new();

    let response = client
        .get(&url)
        .send()?
        .json::<OllamaModelsResponse>()?;

    Ok(response.models)
}

fn format_size(bytes: u64) -> String {
    const GB: u64 = 1024 * 1024 * 1024;
    const MB: u64 = 1024 * 1024;

    if bytes >= GB {
        format!("{:.1}GB", bytes as f64 / GB as f64)
    } else {
        format!("{:.0}MB", bytes as f64 / MB as f64)
    }
}

fn generate_ollama_streaming<F>(
    config: &Config,
    prompt: &str,
    mut on_token: F,
) -> Result<String, Box<dyn std::error::Error>>
where
    F: FnMut(&str),
{
    let url = format!("{}/api/generate", config.ollama_url);
    let client = reqwest::blocking::Client::new();

    let request = OllamaRequest {
        model: config.ollama_model.clone(),
        prompt: prompt.to_string(),
        stream: true,
    };

    let response = client.post(&url).json(&request).send()?;
    let reader = BufReader::new(response);

    let mut full_response = String::new();

    for line in reader.lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }

        if let Ok(chunk) = serde_json::from_str::<OllamaResponse>(&line) {
            full_response.push_str(&chunk.response);
            on_token(&chunk.response);

            if chunk.done {
                break;
            }
        }
    }

    Ok(full_response)
}

fn generate_ollama_quiet(config: &Config, prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    generate_ollama_streaming(config, prompt, |_| {})
}

// ============================================================================
// TUI
// ============================================================================

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

fn run_interactive(
    intent: &str,
    config: &Config,
    prompt: &str,
    file_count: usize,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut stdout = io::stdout();
    let is_tty = atty::is(atty::Stream::Stdout);

    if !is_tty {
        // Non-interactive mode, just generate quietly
        return generate_ollama_quiet(config, prompt);
    }

    // Show context header
    stdout.execute(SetForegroundColor(Color::DarkGrey))?;
    stdout.execute(Print(format!(
        "Using {} · {} files in context\n",
        config.ollama_model, file_count
    )))?;
    stdout.execute(ResetColor)?;

    // Show intent
    stdout.execute(SetForegroundColor(Color::White))?;
    stdout.execute(Print(format!("› {}\n", intent)))?;
    stdout.execute(ResetColor)?;

    // Spinner state
    let spinner_idx = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let spinner_idx_clone = spinner_idx.clone();
    let got_first_token = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let got_first_token_clone = got_first_token.clone();
    let start_time = std::time::Instant::now();

    // Start spinner in background
    let spinner_handle = std::thread::spawn(move || {
        let mut stdout = io::stdout();
        let phases = [
            "Connecting",
            "Waiting for model",
            "Generating",
        ];

        while !got_first_token_clone.load(std::sync::atomic::Ordering::Relaxed) {
            let elapsed = start_time.elapsed().as_secs_f32();
            let phase_idx = match elapsed {
                t if t < 0.5 => 0,
                t if t < 2.0 => 1,
                _ => 2,
            };
            let phase = phases[phase_idx];

            let idx = spinner_idx_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % SPINNER_FRAMES.len();
            let _ = stdout.execute(cursor::MoveToColumn(0));
            let _ = stdout.execute(terminal::Clear(ClearType::CurrentLine));
            let _ = stdout.execute(SetForegroundColor(Color::Cyan));
            let _ = stdout.execute(Print(format!(
                "{} {}... {:.1}s",
                SPINNER_FRAMES[idx],
                phase,
                elapsed
            )));
            let _ = stdout.execute(ResetColor);
            let _ = stdout.flush();
            std::thread::sleep(std::time::Duration::from_millis(80));
        }
    });

    let mut result = String::new();
    let mut first_token = true;
    let gen_start = std::time::Instant::now();

    let generation_result = generate_ollama_streaming(config, prompt, |token| {
        if first_token {
            got_first_token.store(true, std::sync::atomic::Ordering::Relaxed);
            // Clear spinner line
            let _ = stdout.execute(cursor::MoveToColumn(0));
            let _ = stdout.execute(terminal::Clear(ClearType::CurrentLine));
            let _ = stdout.execute(SetForegroundColor(Color::Green));
            let _ = stdout.execute(Print("› "));
            let _ = stdout.execute(ResetColor);
            first_token = false;
        }
        let _ = stdout.execute(Print(token));
        let _ = stdout.flush();
        result.push_str(token);
    });

    // Wait for spinner to stop
    got_first_token.store(true, std::sync::atomic::Ordering::Relaxed);
    let _ = spinner_handle.join();

    let total_time = gen_start.elapsed().as_secs_f32();

    // If we never got a token, clear spinner
    if first_token {
        stdout.execute(cursor::MoveToColumn(0))?;
        stdout.execute(terminal::Clear(ClearType::CurrentLine))?;
    } else {
        // Show timing
        stdout.execute(SetForegroundColor(Color::DarkGrey))?;
        stdout.execute(Print(format!(" ({:.1}s)\n", total_time)))?;
        stdout.execute(ResetColor)?;
    }

    generation_result?;

    Ok(result)
}

// ============================================================================
// Command Sanitizer
// ============================================================================

fn clean_command(raw: &str) -> String {
    let mut cmd = raw.to_string();

    // Remove markdown code blocks
    if cmd.contains("```") {
        // Extract content between ``` markers
        if let Some(start) = cmd.find("```") {
            let after_start = &cmd[start + 3..];
            // Skip language identifier (e.g., ```bash)
            let content_start = after_start.find('\n').map(|i| i + 1).unwrap_or(0);
            let content = &after_start[content_start..];
            if let Some(end) = content.find("```") {
                cmd = content[..end].to_string();
            }
        }
    }

    // Remove inline backticks
    cmd = cmd.replace('`', "");

    // Take only the first line (ignore any explanations)
    cmd = cmd.lines().next().unwrap_or("").to_string();

    // Clean up whitespace
    cmd.replace('\r', "").trim().to_string()
}

// ============================================================================
// Clipboard (for copying command)
// ============================================================================

#[cfg(target_os = "macos")]
fn copy_to_clipboard(text: &str) -> io::Result<()> {
    use std::process::{Command, Stdio};

    let mut child = Command::new("pbcopy")
        .stdin(Stdio::piped())
        .spawn()?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(text.as_bytes())?;
    }

    child.wait()?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn copy_to_clipboard(text: &str) -> io::Result<()> {
    use std::process::{Command, Stdio};

    // Try xclip first, then xsel
    let result = Command::new("xclip")
        .args(["-selection", "clipboard"])
        .stdin(Stdio::piped())
        .spawn()
        .or_else(|_| {
            Command::new("xsel")
                .args(["--clipboard", "--input"])
                .stdin(Stdio::piped())
                .spawn()
        });

    match result {
        Ok(mut child) => {
            if let Some(stdin) = child.stdin.as_mut() {
                stdin.write_all(text.as_bytes())?;
            }
            child.wait()?;
            Ok(())
        }
        Err(e) => Err(e),
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn copy_to_clipboard(_text: &str) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "Clipboard not supported on this platform",
    ))
}

// ============================================================================
// CLI
// ============================================================================

fn print_usage() {
    eprintln!(
        r#"Usage: ai <intent>
       ai config [show|set <key> <value>]
       ai models
       ai init [zsh|bash|fish]

Commands:
  config        - Show or modify configuration
  models        - List available Ollama models
  init          - Install shell integration

Config keys:
  ollama_model  - Ollama model name (default: llama3.2)
  ollama_url    - Ollama API URL (default: http://localhost:11434)

Examples:
  ai "list all files"
  ai config show
  ai config set ollama_model mistral
  ai models
  ai init zsh
"#
    );
}

fn handle_models() {
    let config = Config::load();

    match list_ollama_models(&config) {
        Ok(models) => {
            if models.is_empty() {
                println!("No models found. Pull one with: ollama pull llama3.2");
                return;
            }

            println!("Available models:\n");
            for model in &models {
                let current = if model.name == config.ollama_model || model.name.starts_with(&format!("{}:", config.ollama_model)) {
                    " (current)"
                } else {
                    ""
                };
                println!("  {} ({}){}", model.name, format_size(model.size), current);
            }
            println!("\nSet model with: ai config set ollama_model <name>");
        }
        Err(e) => {
            eprintln!("Failed to list models: {}", e);
            eprintln!("Make sure Ollama is running: ollama serve");
            std::process::exit(1);
        }
    }
}

// ============================================================================
// Shell Integration
// ============================================================================

const ZSH_INTEGRATION: &str = r#"# ai shell integration
_ai_is_safe() {
  local cmd="${1}"
  local lowered="${cmd:l}"
  if [[ "${lowered}" == *"rm -rf /"* || "${lowered}" == *"rm -rf *"* ]]; then
    return 1
  fi
  if [[ "${cmd}" == *\`* ]]; then
    return 1
  fi
  if [[ "${cmd}" == *[$'\000'-$'\037']* ]]; then
    return 1
  fi
  return 0
}

_ai_widget() {
  local intent="${BUFFER}"
  if [[ -z "${intent}" ]]; then
    intent="suggest a useful command for this directory"
  fi

  # Clear current line and run ai with TUI on /dev/tty, capture result
  zle -I  # invalidate display
  echo ""  # newline before ai output

  local suggestion exit_code
  suggestion=$(ai --quick "${intent}" 2>/dev/null </dev/tty)
  exit_code=$?

  case "${exit_code}" in
    0) ;;
    1) zle -M "ai: missing intent"; return ;;
    2) zle -M "ai: blocked dangerous command"; return ;;
    *) zle -M "ai: error (${exit_code})"; return ;;
  esac

  if ! _ai_is_safe "${suggestion}"; then
    zle -M "ai: blocked dangerous command"
    return
  fi

  BUFFER="${suggestion}"
  CURSOR=${#BUFFER}
  zle redisplay
}

zle -N ai-widget _ai_widget
bindkey '^G' ai-widget
"#;

const BASH_INTEGRATION: &str = r#"# ai shell integration
_ai_is_safe() {
  local cmd="$1"
  local lowered="${cmd,,}"
  if [[ "$lowered" == *"rm -rf /"* || "$lowered" == *"rm -rf *"* ]]; then
    return 1
  fi
  if [[ "$cmd" == *\`* ]]; then
    return 1
  fi
  return 0
}

_ai_suggest() {
  local intent="$READLINE_LINE"
  if [[ -z "$intent" ]]; then
    intent="suggest a useful command for this directory"
  fi

  local suggestion exit_code
  suggestion=$(ai "$intent" 2>/dev/null)
  exit_code=$?

  if [[ $exit_code -ne 0 ]]; then
    return
  fi

  if ! _ai_is_safe "$suggestion"; then
    return
  fi

  READLINE_LINE="$suggestion"
  READLINE_POINT=${#READLINE_LINE}
}

bind -x '"\C-g": _ai_suggest'
"#;

const FISH_INTEGRATION: &str = r#"# ai shell integration
function _ai_is_safe
  set -l cmd $argv[1]
  set -l lowered (string lower $cmd)
  if string match -q "*rm -rf /*" $lowered; or string match -q "*rm -rf \\**" $lowered
    return 1
  end
  if string match -q "*\`*" $cmd
    return 1
  end
  return 0
end

function _ai_suggest
  set -l intent (commandline)
  if test -z "$intent"
    set intent "suggest a useful command for this directory"
  end

  set -l suggestion (ai "$intent" 2>/dev/null)
  set -l exit_code $status

  if test $exit_code -ne 0
    return
  end

  if not _ai_is_safe "$suggestion"
    return
  end

  commandline -r "$suggestion"
  commandline -f end-of-line
end

bind \cg _ai_suggest
"#;

fn get_shell_rc_path(shell: &str) -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    match shell {
        "zsh" => Some(home.join(".zshrc")),
        "bash" => {
            let bashrc = home.join(".bashrc");
            let bash_profile = home.join(".bash_profile");
            if bashrc.exists() {
                Some(bashrc)
            } else {
                Some(bash_profile)
            }
        }
        "fish" => Some(home.join(".config/fish/config.fish")),
        _ => None,
    }
}

fn get_integration_content(shell: &str) -> Option<&'static str> {
    match shell {
        "zsh" => Some(ZSH_INTEGRATION),
        "bash" => Some(BASH_INTEGRATION),
        "fish" => Some(FISH_INTEGRATION),
        _ => None,
    }
}

fn handle_init(args: &[String]) {
    let shell = if args.is_empty() {
        // Try to detect shell from SHELL env var
        env::var("SHELL")
            .ok()
            .and_then(|s| s.rsplit('/').next().map(String::from))
            .unwrap_or_else(|| "zsh".to_string())
    } else {
        args[0].clone()
    };

    let integration = match get_integration_content(&shell) {
        Some(content) => content,
        None => {
            eprintln!("Unsupported shell: {}. Supported: zsh, bash, fish", shell);
            std::process::exit(1);
        }
    };

    let rc_path = match get_shell_rc_path(&shell) {
        Some(path) => path,
        None => {
            eprintln!("Could not determine shell config path");
            std::process::exit(1);
        }
    };

    // Write integration file to config dir
    let integration_path = Config::config_dir().join(format!("integration.{}", shell));
    if let Err(e) = fs::create_dir_all(Config::config_dir()) {
        eprintln!("Failed to create config directory: {}", e);
        std::process::exit(1);
    }
    if let Err(e) = fs::write(&integration_path, integration) {
        eprintln!("Failed to write integration file: {}", e);
        std::process::exit(1);
    }

    // Check if already sourced in rc file
    let source_line = format!("source \"{}\"", integration_path.display());
    let rc_content = fs::read_to_string(&rc_path).unwrap_or_default();

    if rc_content.contains(&source_line) || rc_content.contains(integration_path.to_str().unwrap_or("")) {
        println!("Shell integration already installed in {}", rc_path.display());
        println!("\nRun this to reload your shell:");
        println!("  source \"{}\"", rc_path.display());
        return;
    }

    // Append source line to rc file
    let addition = format!("\n# ai\n{}\n", source_line);
    if let Err(e) = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&rc_path)
        .and_then(|mut f| std::io::Write::write_all(&mut f, addition.as_bytes()))
    {
        eprintln!("Failed to update {}: {}", rc_path.display(), e);
        eprintln!("\nManually add this line to your shell config:");
        eprintln!("  {}", source_line);
        std::process::exit(1);
    }

    println!("Installed {} integration to {}", shell, rc_path.display());
    println!("Integration file: {}", integration_path.display());
    println!("\nRun this to activate now:");
    println!("  source \"{}\"", rc_path.display());
    println!("\nThen press Ctrl+G to trigger AI suggestions!");
}

fn handle_config(args: &[String]) {
    let config = Config::load();

    if args.is_empty() || args[0] == "show" {
        println!("Current configuration:");
        println!("  backend:      {}", config.backend);
        println!("  ollama_model: {}", config.ollama_model);
        println!("  ollama_url:   {}", config.ollama_url);
        println!("\nConfig file: {}", Config::config_path().display());
        return;
    }

    if args[0] == "set" {
        if args.len() < 3 {
            eprintln!("Usage: ai config set <key> <value>");
            std::process::exit(1);
        }

        let key = &args[1];
        let value = &args[2];
        let mut new_config = config;

        match key.as_str() {
            "backend" => match value.parse::<Backend>() {
                Ok(backend) => new_config.backend = backend,
                Err(e) => {
                    eprintln!("{}", e);
                    std::process::exit(1);
                }
            },
            "ollama_model" => new_config.ollama_model = value.clone(),
            "ollama_url" => new_config.ollama_url = value.clone(),
            _ => {
                eprintln!("Unknown config key: {}", key);
                std::process::exit(1);
            }
        }

        if let Err(e) = new_config.save() {
            eprintln!("Failed to save config: {}", e);
            std::process::exit(1);
        }
        println!("Set {} = {}", key, value);
        return;
    }

    eprintln!("Unknown config command: {}", args[0]);
    std::process::exit(1);
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        print_usage();
        std::process::exit(1);
    }

    // Check for --quick flag (quiet mode for shell integration)
    let quick_mode = args.iter().any(|a| a == "--quick" || a == "-q");
    let args: Vec<String> = args.into_iter().filter(|a| a != "--quick" && a != "-q").collect();

    if args.is_empty() {
        print_usage();
        std::process::exit(1);
    }

    // Handle subcommands
    match args[0].as_str() {
        "-h" | "--help" | "help" => {
            print_usage();
            return;
        }
        "-v" | "--version" | "version" => {
            println!("ai {}", env!("CARGO_PKG_VERSION"));
            return;
        }
        "config" => {
            handle_config(&args[1..]);
            return;
        }
        "models" => {
            handle_models();
            return;
        }
        "init" => {
            handle_init(&args[1..]);
            return;
        }
        _ => {}
    }

    let intent = args.join(" ").trim().to_string();
    let working_directory = env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let files = collect_files();
    let file_count = files.len();
    let prompt = build_prompt(&intent, &working_directory, &files);

    let config = Config::load();

    let raw = if quick_mode {
        // Quick mode: no TUI, just output the command
        match generate_ollama_quiet(&config, &prompt) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("model error: {}", e);
                std::process::exit(3);
            }
        }
    } else {
        // Interactive mode with TUI
        match run_interactive(&intent, &config, &prompt, file_count) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("model error: {}", e);
                std::process::exit(3);
            }
        }
    };

    let command = clean_command(&raw);
    if command.is_empty() || !is_safe(&command) {
        std::process::exit(2);
    }

    // In quick mode or non-TTY, print the command to stdout
    if quick_mode || !atty::is(atty::Stream::Stdout) {
        println!("{}", command);
    } else {
        // Interactive mode: copy to clipboard
        if copy_to_clipboard(&command).is_ok() {
            let mut stdout = io::stdout();
            let _ = stdout.execute(SetForegroundColor(Color::DarkGrey));
            let _ = stdout.execute(Print("Copied to clipboard. Press Cmd+V to paste.\n"));
            let _ = stdout.execute(ResetColor);
        }
    }
}
