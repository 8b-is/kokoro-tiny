//! Example showing how to list and select audio output devices

use kokoro_tiny::TtsEngine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎤 kokoro-tiny device selection example");
    println!("==========================================\n");

    // Initialize TTS engine
    let mut tts = TtsEngine::new().await?;

    // List available audio devices
    #[cfg(feature = "playback")]
    {
        println!("📻 Available audio output devices:");
        let devices = tts.list_audio_devices()?;
        for (i, device) in devices.iter().enumerate() {
            println!("  {}. {}", i + 1, device);
        }
        println!();

        // Test with each device
        let text = "Testing audio output on this device.";

        for device in &devices {
            println!("🔊 Playing on: {}", device);

            // Set the device
            tts.set_audio_device(Some(device.clone()))?;

            // Synthesize and play
            let audio = tts.synthesize(text, None)?;
            tts.play(&audio, 0.8)?;

            println!("✅ Playback complete\n");

            // Small delay between devices
            std::thread::sleep(std::time::Duration::from_millis(500));
        }

        // Reset to default
        println!("🔄 Resetting to system default device");
        tts.set_audio_device(None)?;
        let audio = tts.synthesize("Back to default device.", None)?;
        tts.play(&audio, 0.8)?;
    }

    #[cfg(not(feature = "playback"))]
    {
        println!("⚠️  Playback feature not enabled. Rebuild with --features playback");
    }

    Ok(())
}
