//! Test different text formats to fix volume issues

use kokoro_tiny::TtsEngine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎤 Testing different text formats for consistent volume...\n");

    let mut tts = TtsEngine::new().await
        .map_err(|e| format!("Failed to initialize TTS: {}", e))?;

    // Test different text variations
    let test_messages = vec![
        ("test1", "Hello Alexandra and Christopher Chenoweth!"),
        ("test2", "Hello, Alexandra and Christopher Chenoweth."),
        ("test3", "Hello alexandra and christopher chenoweth"),
        ("test4", "Hello, alexandra and christopher chenoweth."),
        ("test5", "Hello. Alexandra and Christopher Chenoweth."),
        ("test6", "Hello: Alexandra and Christopher Chenoweth"),
    ];

    for (filename, message) in test_messages {
        println!("Testing: \"{}\"", message);

        let audio = tts.synthesize(message, Some("af_heart"), None)
            .map_err(|e| format!("Synthesis failed: {}", e))?;

        let output_file = format!("{}.wav", filename);
        tts.save_wav(&output_file, &audio)
            .map_err(|e| format!("Failed to save: {}", e))?;

        println!("  ✅ Saved to {}", output_file);
    }

    println!("\n🎉 All test files generated! Listen to find the best format.");
    Ok(())
}