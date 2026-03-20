//! Test with explicit American voice

use kokoro_tiny::TtsEngine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎤 Testing American English voice...\n");

    let mut tts = TtsEngine::new().await?;

    let text = "Hello. My name is Adam. This is a test of American English.";

    println!("Text: \"{}\"", text);
    println!("Voice: am_adam (American Male)\n");

    let audio = tts.synthesize_with_speed(text, Some("am_adam"), 1.0, None)?;

    tts.save_wav("american_test.wav", &audio)?;
    println!("✅ Saved to american_test.wav");

    #[cfg(feature = "playback")]
    {
        println!("🔊 Playing...\n");
        tts.play(&audio, 0.9)?;
    }

    Ok(())
}
