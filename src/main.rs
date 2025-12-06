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
    fn config_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_default()
            .join(".config")
            .join("aisuggest")
            .join("config.json")
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
        r#"Usage: aisuggest <intent>
       aisuggest config [show|set <key> <value>]

Config keys:
  backend       - 'ollama'
  ollama_model  - Ollama model name (default: llama3.2)
  ollama_url    - Ollama API URL (default: http://localhost:11434)

Examples:
  aisuggest "list all files"
  aisuggest config show
  aisuggest config set ollama_model mistral
"#
    );
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
            eprintln!("Usage: aisuggest config set <key> <value>");
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

    // Handle config subcommand
    if args[0] == "config" {
        handle_config(&args[1..]);
        return;
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
