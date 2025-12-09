use assert_cmd::Command;
use uuid::Uuid;
use std::time::Instant;

#[test]
// #[ignore] -- Enabled by default now for real testing
fn test_heavy_generation_no_cache() {
    let mut cmd = Command::cargo_bin("mate").unwrap();
    
    // Generate a unique ID to ensure the prompt is novel and bypasses any Ollama prompts cache
    let unique_id = Uuid::new_v4().to_string();
    let prompt = format!("Say exactly this uuid: {}", unique_id);
    
    println!("Running heavy integration test with prompt: '{}'", prompt);
    let start = Instant::now();
    
    let assert = cmd.arg(&prompt)
        // Force simple mode to avoid router overhead/confusion for this specific test
        // or just let it route. A simple "Say X" usually routes to empty context.
        // Let's use --quick to hit the model directly and skip the router for speed/simplicity of this specific test.
        .arg("--quick") 
        .assert();
        
    let duration = start.elapsed();
    
    let output = assert.success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    
    println!("Model execution took: {:?}", duration);
    println!("Output: {}", stdout);
    
    // 1. Assert correctness
    assert!(stdout.contains(&unique_id), "Model failed to return the unique UUID");
    
    // 2. Assert timing (Real models take time)
    // Local models might be fast on M-series chips, but usually > 50ms implies no simple lookup.
    // However, if caching WAS hit, it would be sub-20ms potentially. 
    // Since prompt is unique, we expect full inference. 
    // This assertion is advisory but good for manual verification.
    assert!(duration.as_millis() > 100, "Execution was suspiciously fast ({}ms). Cached?", duration.as_millis());
}
