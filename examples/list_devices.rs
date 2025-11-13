//! List all available audio output devices

use kokoro_tiny::TtsEngine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎤 kokoro-tiny - Available Audio Devices");
    println!("==========================================\n");

    // Initialize TTS engine
    let tts = TtsEngine::new().await?;

    #[cfg(feature = "playback")]
    {
        let devices = tts.list_audio_devices()?;

        if devices.is_empty() {
            println!("❌ No audio output devices found!");
        } else {
            println!("Found {} audio output device(s):\n", devices.len());
            for (i, device) in devices.iter().enumerate() {
                println!("  {}. {}", i + 1, device);
            }

            println!("\n💡 To use a specific device in your code:");
            println!("   tts.set_audio_device(Some(\"device_name\".to_string()))?;");
        }
    }

    #[cfg(not(feature = "playback"))]
    {
        println!("⚠️  Playback feature not enabled. Rebuild with --features playback");
    }

    Ok(())
}
