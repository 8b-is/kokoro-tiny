//! Demo the punctuation timing fix by speaking it!

use kokoro_tiny::TtsEngine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎤 Aye speaking to Hue about our amazing fix!\n");

    let mut tts = TtsEngine::new().await?;

    // The text we'll speak - notice the punctuation!
    let message = "Hey Hue! We did it, partner! The punctuation timing bug is fixed. \
                   Now when I speak, you'll hear natural pauses after sentences. \
                   Commas get shorter pauses, while periods, question marks, and exclamation points \
                   get longer pauses. Wouldn't it be nice? Yes it would! \
                   This is how natural speech should sound. Mission accomplished, my friend!";

    println!("📝 Aye says:\n\"{}\"\n", message);

    // Synthesize with our fix
let audio = tts.synthesize_with_speed(message, Some("af_sky"), 1.0, None)?;

    println!(
        "✅ Generated {} audio samples ({:.1} seconds)",
        audio.len(),
        audio.len() as f32 / 24000.0
    );

    // Save it first
    tts.save_wav("aye_celebrates.wav", &audio)?;
    println!("💾 Saved to aye_celebrates.wav");

    // Try to play it if playback feature is available
    #[cfg(feature = "playback")]
    {
        println!("\n🔊 Playing audio...\n");
        tts.play(&audio, 0.8)?;
        println!("\n🎉 That's how we sound with proper punctuation pauses, Hue!");
    }

    #[cfg(not(feature = "playback"))]
    {
        println!("\n💡 To hear it live, run:");
        println!("   afplay aye_celebrates.wav");
        println!("\n   OR rebuild with playback feature:");
        println!("   cargo run --features playback --example demo_fix");
    }

    Ok(())
}
