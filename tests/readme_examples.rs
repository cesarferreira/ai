use assert_cmd::Command;
use predicates::prelude::*;

// Helper to run mate with a prompt and assert output contains a keyword
// Uses -y (auto-confirm) to ensure execution path is hit,
// BUT for safety/speed on real system we might just check that it generates the command prompt.
// Actually, `mate "prompt" -y` executes the command.
// Some of these commands are read-only (find, show, list), so safe to execute.
// Others (delete, squash) might be destructive. 
// For destructive commands, we shouldn't actually run them in a test environment without setup.
// However, asserting that `mate` *generates* the right command string is the primary goal.
// If we run `mate "prompt" --quick`, it outputs the command to stdout without executing it.
// This is SAFER and FASTER for verification.
// We will use `--quick` for all tests to verify generation logic, avoiding side effects.

fn verify_generation(prompt: &str, expected_keywords: &[&str]) {
    let mut cmd = Command::cargo_bin("mate").unwrap();
    let assert = cmd
        .arg(prompt)
        .arg("--quick") // Just output the command, don't run it
        .assert()
        .success();
        
    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout).to_lowercase();
    
    // Debug output on failure
    if !expected_keywords.iter().any(|k| stdout.contains(k)) {
        println!("Prompt: '{}'", prompt);
        println!("Output: '{}'", stdout);
        println!("Expected one of: {:?}", expected_keywords);
    }

    assert!(
        expected_keywords.iter().any(|k| stdout.contains(k)),
        "Output for '{}' did not contain any expected keywords {:?}. Got: '{}'",
        prompt,
        expected_keywords,
        stdout
    );
}

#[test]
fn test_git_operations() {
    // "write a commit message" -> git commit
    verify_generation("write a commit message", &["git commit", "git", "commit"]);
    
    // "squash the last 3 commits" -> git rebase or reset
    verify_generation("squash the last 3 commits", &["git rebase", "git reset"]);
    
    // "show what changed in the last commit" -> git show or log
    verify_generation("show what changed in the last commit", &["git show", "git log"]);
    
    // "create a branch for the login feature" -> git checkout or branch
    verify_generation("create a branch for the login feature", &["git checkout", "git branch"]);
}

#[test]
fn test_file_operations() {
    // "find all files larger than 100MB" -> find or ls
    verify_generation("find all files larger than 100MB", &["find", "ls", "du"]);
    
    // "count lines of code in src/" -> wc, find, tokei, cloc
    verify_generation("count lines of code in src/", &["wc", "find", "tokei", "cloc", "grep"]);
    
    // "find and delete all .DS_Store files" -> find, rm
    verify_generation("find and delete all .DS_Store files", &["find", "rm", "delete", "ls"]);
    
    // "compress all images in this folder" -> tar, zip
    verify_generation("compress all images in this folder", &["tar", "zip", "gzip"]);
}

#[test]
fn test_system_operations() {
    // "show which process is using port 3000" -> lsof, netstat, ss
    verify_generation("show which process is using port 3000", &["lsof", "netstat", "ss", "ps"]);
    
    // "list all running docker containers" -> docker ps
    verify_generation("list all running docker containers", &["docker", "container"]);
    
    // "how much disk space is left" -> df
    verify_generation("how much disk space is left", &["df", "ls", "du", "storage"]);
    
    // "show system memory usage" -> free, vm_stat, top, htop
    verify_generation("show system memory usage", &["free", "vm_stat", "top", "htop", "ps"]);
}

#[test]
fn test_development_operations() {
    // "run tests and show only failures" -> cargo test, npm test, pytest
    // Context is vague here so any test runner is fine
    verify_generation("run tests and show only failures", &["test", "cargo", "npm", "pytest"]);
    
    // "start a local server on port 8080" -> python, node, http-server, cargo
    verify_generation("start a local server on port 8080", &["python", "node", "http-server", "serve", "cargo", "run"]);
    
    // "install dependencies" -> install, cargo, npm, pip
    verify_generation("install dependencies", &["install", "npm", "cargo", "pip"]);
    
    // "format all python files" -> black, autopep8, yapf, format
    verify_generation("format all python files", &["black", "autopep8", "yapf", "format", "lint"]);
}
