import Foundation

struct SafetyFilter {
    static func isSafe(_ command: String) -> Bool {
        let lowered = command.lowercased()
        if lowered.contains("rm -rf /") { return false }
        if lowered.contains("rm -rf *") { return false }
        if command.contains("`") { return false }
        if containsControlCharacters(command) { return false }
        return true
    }

    private static func containsControlCharacters(_ command: String) -> Bool {
        for scalar in command.unicodeScalars {
            if scalar.value < 0x20 {
                return true
            }
        }
        return false
    }
}

struct PromptBuilder {
    static func build(intent: String, workingDirectory: String, files: [String]) -> String {
        let fileList = files.joined(separator: "\n")
        return """
        You are a CLI assistant. Convert the user's intent into a single safe shell command.

        Current directory: \(workingDirectory)
        Files:
        \(fileList)

        User intent: "\(intent)"

        Rules:
        - Respond with ONE shell command only.
        - No markdown.
        - No explanation.
        - No prose.
        - Favor safe operations.
        """
    }
}

struct FileContextCollector {
    func collect() -> [String] {
        let fm = FileManager.default
        let path = fm.currentDirectoryPath
        guard let items = try? fm.contentsOfDirectory(atPath: path) else { return [] }
        return items.sorted()
    }
}

protocol CommandGenerating {
    func generate(prompt: String) async throws -> String
}

import FoundationModels

@available(macOS 26.0, *)
struct AppleAIGenerator: CommandGenerating {
    func generate(prompt: String) async throws -> String {
        let session = LanguageModelSession()
        let response = try await session.respond(to: prompt)
        return response.content
    }
}

enum ModelError: Error {
    case unavailable
    case ollamaError(String)
}

// MARK: - Ollama Backend

struct OllamaGenerator: CommandGenerating {
    let model: String
    let baseURL: String

    init(model: String = "llama3.2", baseURL: String = "http://localhost:11434") {
        self.model = model
        self.baseURL = baseURL
    }

    func generate(prompt: String) async throws -> String {
        guard let url = URL(string: "\(baseURL)/api/generate") else {
            throw ModelError.ollamaError("Invalid URL")
        }

        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")

        let body: [String: Any] = [
            "model": model,
            "prompt": prompt,
            "stream": false
        ]
        request.httpBody = try JSONSerialization.data(withJSONObject: body)

        let (data, response) = try await URLSession.shared.data(for: request)

        guard let httpResponse = response as? HTTPURLResponse, httpResponse.statusCode == 200 else {
            throw ModelError.ollamaError("Ollama request failed")
        }

        guard let json = try JSONSerialization.jsonObject(with: data) as? [String: Any],
              let responseText = json["response"] as? String else {
            throw ModelError.ollamaError("Invalid response format")
        }

        return responseText
    }
}

// MARK: - Configuration

enum AIBackend: String, CaseIterable {
    case apple
    case ollama
}

struct Config {
    static let configPath = FileManager.default.homeDirectoryForCurrentUser
        .appendingPathComponent(".config/aisuggest/config.json")

    var backend: AIBackend
    var ollamaModel: String
    var ollamaURL: String

    static func load() -> Config {
        guard FileManager.default.fileExists(atPath: configPath.path),
              let data = try? Data(contentsOf: configPath),
              let json = try? JSONSerialization.jsonObject(with: data) as? [String: String] else {
            return Config(backend: .apple, ollamaModel: "llama3.2", ollamaURL: "http://localhost:11434")
        }

        let backend = AIBackend(rawValue: json["backend"] ?? "apple") ?? .apple
        let ollamaModel = json["ollama_model"] ?? "llama3.2"
        let ollamaURL = json["ollama_url"] ?? "http://localhost:11434"

        return Config(backend: backend, ollamaModel: ollamaModel, ollamaURL: ollamaURL)
    }

    func save() throws {
        let dir = Config.configPath.deletingLastPathComponent()
        try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)

        let json: [String: String] = [
            "backend": backend.rawValue,
            "ollama_model": ollamaModel,
            "ollama_url": ollamaURL
        ]
        let data = try JSONSerialization.data(withJSONObject: json, options: .prettyPrinted)
        try data.write(to: Config.configPath)
    }
}

struct ModelClient: CommandGenerating {
    let config: Config

    init(config: Config = .load()) {
        self.config = config
    }

    func generate(prompt: String) async throws -> String {
        switch config.backend {
        case .apple:
            guard #available(macOS 26.0, *) else { throw ModelError.unavailable }
            return try await AppleAIGenerator().generate(prompt: prompt)
        case .ollama:
            return try await OllamaGenerator(
                model: config.ollamaModel,
                baseURL: config.ollamaURL
            ).generate(prompt: prompt)
        }
    }
}

struct CommandSanitizer {
    static func clean(_ raw: String) -> String {
        raw
            .replacingOccurrences(of: "\n", with: " ")
            .replacingOccurrences(of: "\r", with: " ")
            .trimmingCharacters(in: .whitespacesAndNewlines)
    }
}

@main
struct AISuggestCLI {
    static func main() async {
        let args = Array(CommandLine.arguments.dropFirst())
        guard !args.isEmpty else {
            printUsage()
            exit(1)
        }

        // Handle config subcommand
        if args[0] == "config" {
            handleConfig(Array(args.dropFirst()))
            return
        }

        let intent = args.joined(separator: " ").trimmingCharacters(in: .whitespacesAndNewlines)
        let workingDirectory = FileManager.default.currentDirectoryPath
        let files = FileContextCollector().collect()
        let prompt = PromptBuilder.build(intent: intent, workingDirectory: workingDirectory, files: files)

        let generator = ModelClient()
        let raw: String
        do {
            raw = try await generator.generate(prompt: prompt)
        } catch {
            fputs("model error: \(error)\n", stderr)
            exit(3)
        }

        let command = CommandSanitizer.clean(raw)
        guard !command.isEmpty, SafetyFilter.isSafe(command) else {
            exit(2)
        }

        print(command)
    }

    static func printUsage() {
        fputs("""
        Usage: aisuggest <intent>
               aisuggest config [show|set <key> <value>]

        Config keys:
          backend       - 'apple' or 'ollama'
          ollama_model  - Ollama model name (default: llama3.2)
          ollama_url    - Ollama API URL (default: http://localhost:11434)

        Examples:
          aisuggest "list all files"
          aisuggest config show
          aisuggest config set backend ollama
          aisuggest config set ollama_model mistral

        """, stderr)
    }

    static func handleConfig(_ args: [String]) {
        let config = Config.load()

        if args.isEmpty || args[0] == "show" {
            print("Current configuration:")
            print("  backend:      \(config.backend.rawValue)")
            print("  ollama_model: \(config.ollamaModel)")
            print("  ollama_url:   \(config.ollamaURL)")
            print("\nConfig file: \(Config.configPath.path)")
            return
        }

        if args[0] == "set" {
            guard args.count >= 3 else {
                fputs("Usage: aisuggest config set <key> <value>\n", stderr)
                exit(1)
            }

            let key = args[1]
            let value = args[2]
            var newConfig = config

            switch key {
            case "backend":
                guard let backend = AIBackend(rawValue: value) else {
                    fputs("Invalid backend. Use 'apple' or 'ollama'\n", stderr)
                    exit(1)
                }
                newConfig.backend = backend
            case "ollama_model":
                newConfig.ollamaModel = value
            case "ollama_url":
                newConfig.ollamaURL = value
            default:
                fputs("Unknown config key: \(key)\n", stderr)
                exit(1)
            }

            do {
                try newConfig.save()
                print("Set \(key) = \(value)")
            } catch {
                fputs("Failed to save config: \(error)\n", stderr)
                exit(1)
            }
            return
        }

        fputs("Unknown config command: \(args[0])\n", stderr)
        exit(1)
    }
}
