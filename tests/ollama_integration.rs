use assert_cmd::Command;

fn run_quick(prompt: &str) -> String {
    let assert = Command::cargo_bin("mate")
        .unwrap()
        .arg(prompt)
        .arg("--quick")
        .assert()
        .success();

    String::from_utf8_lossy(&assert.get_output().stdout)
        .trim()
        .to_string()
}

#[test]
fn models_command_reaches_ollama() {
    let assert = Command::cargo_bin("mate")
        .unwrap()
        .arg("models")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).to_lowercase();
    assert!(
        stdout.contains("model"),
        "Expected `mate models` to hit the Ollama endpoint and mention models. Output: {}",
        stdout
    );
}

#[test]
fn readme_todo_example_generates_search_command() {
    let output = run_quick("find all TODO comments in this project").to_lowercase();
    assert!(
        output.contains("rg") || output.contains("grep") || output.contains("find"),
        "Expected a search-style command for TODOs, got: {}",
        output
    );
}

#[test]
fn readme_commit_message_example_generates_git_commit() {
    let output = run_quick("write a commit message for my changes").to_lowercase();
    assert!(
        output.contains("git commit"),
        "Expected a git commit command similar to the README example, got: {}",
        output
    );
}

#[test]
fn quick_mode_outputs_single_shell_line() {
    let raw = Command::cargo_bin("mate")
        .unwrap()
        .arg("list files modified today")
        .arg("--quick")
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&raw.get_output().stdout).to_string();
    let lines: Vec<_> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();

    assert_eq!(lines.len(), 1, "Expected a single-line command, got: {}", stdout);
    assert!(
        !stdout.contains('`'),
        "Quick mode should not wrap the command in backticks. Output: {}",
        stdout
    );
}
