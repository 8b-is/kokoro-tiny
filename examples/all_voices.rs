//! Test all English voices with the same phrase

use kokoro_tiny::TtsEngine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎤 Testing all English voices");
    println!("================================\n");

    let mut tts = TtsEngine::new().await?;

    let text = "And the smell of rain out on the farm as it feels like the world is being refreshed.";

    // Get all available voices
    let mut voices: Vec<String> = tts.voices();
    voices.sort();

    // Filter to English voices (start with 'a' for American/British female/male)
    let english_voices: Vec<String> = voices.iter()
        .filter(|v| v.starts_with("af_") || v.starts_with("am_") || v.starts_with("bf_") || v.starts_with("bm_"))
        .cloned()
        .collect();

    println!("Found {} English voices:\n", english_voices.len());

    for voice in &english_voices {
        println!("🔊 Voice: {}", voice);

        match tts.synthesize(text, Some(voice)) {
            Ok(audio) => {
                let duration_secs = audio.len() as f32 / 24000.0;
                println!("   Duration: {:.1}s ({} samples)", duration_secs, audio.len());

                #[cfg(feature = "playback")]
                {
                    if let Err(e) = tts.play(&audio, 0.8) {
                        println!("   ❌ Playback error: {}", e);
                    } else {
                        println!("   ✅ Played");
                    }
                }

                #[cfg(not(feature = "playback"))]
                {
                    println!("   ✅ Synthesized");
                }
            }
            Err(e) => println!("   ❌ Error: {}", e),
        }

        // Longer pause between voices so they don't overlap
        std::thread::sleep(std::time::Duration::from_secs(2));
        println!();
    }

    Ok(())
}
