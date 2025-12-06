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
}

struct ModelClient: CommandGenerating {
    func generate(prompt: String) async throws -> String {
        guard #available(macOS 26.0, *) else { throw ModelError.unavailable }
        return try await AppleAIGenerator().generate(prompt: prompt)
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
            exit(1)
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
}
