//! Simple example showing basic TTS usage

use kokoro_tiny::TtsEngine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎤 kokoro-tiny simple example");
    println!("==============================\n");

    // Initialize TTS engine
    let mut tts = TtsEngine::new().await?;

    // Simple text synthesis
    let text = "Hello from kokoro-tiny! This is a minimal text to speech engine.";
    println!("Synthesizing: \"{}\"\n", text);

    let audio = tts.synthesize(text, None)?;
    println!("✅ Generated {} audio samples", audio.len());

    // Save to file
    tts.save_wav("simple_output.wav", &audio)?;
    println!("💾 Saved to: simple_output.wav");

    // Play if feature enabled
    #[cfg(feature = "playback")]
    {
        println!("🔊 Playing audio...");
        tts.play(&audio, 0.8)?;
    }

    Ok(())
}
