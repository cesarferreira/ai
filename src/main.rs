use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
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
    if command.contains('`') {
        return false;
    }
    if command.chars().any(|c| (c as u32) < 0x20 && c != '\t') {
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
        r#"You are a CLI assistant. Convert the user's intent into a single safe shell command.

Current directory: {}
Files:
{}

User intent: "{}"

Rules:
- Respond with ONE shell command only.
- No markdown.
- No explanation.
- No prose.
- Favor safe operations."#,
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
        Self::config_dir().join("config.json")
    }

    fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            fs::read_to_string(&path)
                .ok()
                .and_then(|content| serde_json::from_str(&content).ok())
                .unwrap_or_default()
        } else {
            Config::default()
        }
    }

    fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
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
}

fn generate_ollama(config: &Config, prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    let url = format!("{}/api/generate", config.ollama_url);
    let client = reqwest::blocking::Client::new();

    let request = OllamaRequest {
        model: config.ollama_model.clone(),
        prompt: prompt.to_string(),
        stream: false,
    };

    let response = client
        .post(&url)
        .json(&request)
        .send()?
        .json::<OllamaResponse>()?;

    Ok(response.response)
}

// ============================================================================
// Command Sanitizer
// ============================================================================

fn clean_command(raw: &str) -> String {
    raw.replace('\n', " ")
        .replace('\r', " ")
        .trim()
        .to_string()
}

// ============================================================================
// CLI
// ============================================================================

fn print_usage() {
    eprintln!(
        r#"Usage: ai <intent>
       ai config [show|set <key> <value>]
       ai init [zsh|bash|fish]

Commands:
  config        - Show or modify configuration
  init          - Install shell integration

Config keys:
  backend       - 'ollama'
  ollama_model  - Ollama model name (default: llama3.2)
  ollama_url    - Ollama API URL (default: http://localhost:11434)

Examples:
  ai "list all files"
  ai config show
  ai config set ollama_model mistral
  ai init zsh
"#
    );
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

  local suggestion status
  suggestion=$(ai "${intent}" 2>/dev/null)
  status=$?

  case "${status}" in
    0) ;;
    1) zle -M "ai: missing intent"; return ;;
    2) zle -M "ai: blocked dangerous command"; return ;;
    *) zle -M "ai: error (${status})"; return ;;
  esac

  if ! _ai_is_safe "${suggestion}"; then
    zle -M "ai: blocked dangerous command"
    return
  fi

  BUFFER="${suggestion}"
  CURSOR=${#BUFFER}
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

  local suggestion status
  suggestion=$(ai "$intent" 2>/dev/null)
  status=$?

  if [[ $status -ne 0 ]]; then
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
  set -l status_code $status

  if test $status_code -ne 0
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

    // Handle subcommands
    match args[0].as_str() {
        "config" => {
            handle_config(&args[1..]);
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
    let prompt = build_prompt(&intent, &working_directory, &files);

    let config = Config::load();
    let raw = match config.backend {
        Backend::Ollama => match generate_ollama(&config, &prompt) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("model error: {}", e);
                std::process::exit(3);
            }
        },
    };

    let command = clean_command(&raw);
    if command.is_empty() || !is_safe(&command) {
        std::process::exit(2);
    }

    println!("{}", command);
}
