//! Simple example showing how to use kokoro-tiny for TTS
//!
//! Run with: cargo run --example simple

use kokoro_tiny::TtsEngine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎤 Initializing Kokoro TTS Engine...");

    // Initialize the TTS engine (auto-downloads model if needed)
    let mut tts = TtsEngine::new().await?;

    // List available voices
    println!("📢 Available voices:");
    for voice in tts.voices() {
        println!("  - {}", voice);
    }

    // Text to synthesize
    let text = "Hello! This is kokoro-tiny, a minimal TTS engine perfect for embedding in your applications. \
                It's fast, lightweight, and produces great quality speech!";

    println!("\n🎵 Synthesizing: \"{}\"", text);

    // Generate speech with default voice
    let audio = tts.synthesize(text, None, None)?;

    println!("✅ Generated {} audio samples", audio.len());

    // Save to WAV file
    tts.save_wav("output.wav", &audio)?;
    println!("💾 Saved to output.wav");

    // Save to MP3 if feature is enabled
    #[cfg(feature = "mp3")]
    {
        tts.save_mp3("output.mp3", &audio)?;
        println!("💾 Saved to output.mp3");
    }

    println!("\n🎉 Done! Check output.wav for the result.");

    Ok(())
}