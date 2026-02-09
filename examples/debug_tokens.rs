//! Debug tokenization process

use kokoro_tiny::TtsEngine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var(
        "PIPER_ESPEAKNG_DATA_DIRECTORY",
        "/opt/homebrew/Cellar/espeak-ng/1.52.0/share",
    );

    println!("🔍 Debugging tokenization process\n");

    let mut tts = TtsEngine::new().await?;

    let text = "Hello";

    println!("Input text: \"{}\"", text);
    println!("Vocab size: {} symbols\n", tts.voices().len());

    // The library doesn't expose tokenize publicly, so let's trace what happens
    println!("Testing with actual synthesis...");

    match tts.synthesize_with_speed(text, Some("af_sky"), 1.0) {
        Ok(audio) => {
            println!("✅ Synthesis succeeded!");
            println!(
                "   Audio length: {} samples ({:.1}s)",
                audio.len(),
                audio.len() as f32 / 24000.0
            );
        }
        Err(e) => {
            println!("❌ Synthesis failed: {}", e);
        }
    }

    Ok(())
}
