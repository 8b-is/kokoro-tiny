//! Fun example for Alexandra and Christopher!
//! Using the Heart voice for a warm greeting

use kokoro_tiny::TtsEngine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎤 Starting Kokoro TTS - Special greeting edition!");

    // Fire up our TTS engine
    let mut tts = TtsEngine::new().await
        .map_err(|e| format!("Whoops, couldn't start TTS: {}", e))?;

    // The special message for Alexandra and Christopher!
    // Using lowercase for more consistent volume
    let message = "hello, alexandra and christopher chenoweth.";

    println!("💝 Using the 'af_heart' voice for warmth!");
    println!("📝 Message: \"{}\"", message);

    // Generate speech with the Heart voice (af_heart)
    let audio = tts.synthesize(message, Some("af_heart"), None)
        .map_err(|e| format!("Speech synthesis hiccup: {}", e))?;

    println!("✅ Generated {} audio samples!", audio.len());

    // Save it with a special filename
    let filename = "hello_alexandra_christopher.wav";
    tts.save_wav(filename, &audio)
        .map_err(|e| format!("Couldn't save audio: {}", e))?;

    println!("💾 Saved to {} - Ready to play!", filename);
    println!("🎉 Success! The Chenoweth family greeting is ready!");

    Ok(())
}