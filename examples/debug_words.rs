//! Debug word dropping issue

use kokoro_tiny::TtsEngine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Debugging word dropping issue");
    println!("=====================================\n");

    // Initialize TTS engine
    let mut tts = TtsEngine::new().await?;

    // Test phrases that might drop words
    let test_phrases = vec![
        "Let me tell you son",
        "Let me",
        "me tell you",
        "tell you son",
        "Hello world this is a test",
        "The quick brown fox",
    ];

    for (i, phrase) in test_phrases.iter().enumerate() {
        println!("Test {}: \"{}\"", i + 1, phrase);

        // Synthesize at different speeds
        for speed in &[0.5, 0.85, 1.0] {
            println!("  Speed {}x:", speed);

            match tts.synthesize_with_speed(phrase, None, *speed, None) {
                Ok(audio) => {
                    let filename = format!("debug_{}_speed_{}.wav", i + 1, (speed * 100.0) as u32);
                    tts.save_wav(&filename, &audio)?;
                    println!("    ✅ Saved to {} ({} samples)", filename, audio.len());

                    // Also play it
                    #[cfg(feature = "playback")]
                    {
                        if let Err(e) = tts.play(&audio, 0.8) {
                            println!("    ⚠️  Playback error: {}", e);
                        }
                        std::thread::sleep(std::time::Duration::from_millis(500));
                    }
                }
                Err(e) => println!("    ❌ Synthesis failed: {}", e),
            }
        }
        println!();
    }

    Ok(())
}
