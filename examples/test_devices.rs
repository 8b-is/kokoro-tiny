//! Test specific audio devices (Scarlett and LG TV)

use kokoro_tiny::TtsEngine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎤 Testing specific audio devices");
    println!("===================================\n");

    // Initialize TTS engine
    #[cfg_attr(not(feature = "playback"), allow(unused_mut))]
    let mut tts = TtsEngine::new().await?;

    #[cfg(feature = "playback")]
    {
        // Test devices
        let test_devices = vec![
            ("Scarlett 18i20", "sysdefault:CARD=USB"),
            ("Scarlett front", "front:CARD=USB,DEV=0"),
            ("LG TV HDMI", "hdmi:CARD=NVidia,DEV=0"),
            ("System default", "default"),
            ("PulseAudio", "pulse"),
        ];

        for (name, device) in test_devices {
            println!("🔊 Testing: {}", name);
            println!("   Device: {}", device);

            match tts.set_audio_device(Some(device.to_string())) {
                Ok(_) => {
                    let text = format!("Testing {} output.", name);
                    match tts.synthesize(&text, None) {
                        Ok(audio) => match tts.play(&audio, 0.9) {
                            Ok(_) => println!("   ✅ Playback successful\n"),
                            Err(e) => println!("   ❌ Playback failed: {}\n", e),
                        },
                        Err(e) => println!("   ❌ Synthesis failed: {}\n", e),
                    }
                }
                Err(e) => println!("   ❌ Device not found: {}\n", e),
            }

            // Small delay between tests
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }

    #[cfg(not(feature = "playback"))]
    {
        println!("⚠️  Playback feature not enabled. Rebuild with --features playback");
    }

    Ok(())
}
