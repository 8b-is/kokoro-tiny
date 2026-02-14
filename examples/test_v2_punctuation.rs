//! Test if v2 branch already handles punctuation timing

use kokoro_tiny::TtsEngine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing punctuation timing on voice-improvements-v2\n");

    let mut tts = TtsEngine::new().await?;

    let text = "Hello there! This is a test. We need longer text to see if pauses work.";

    println!("Text: \"{}\"", text);

    let audio = tts.synthesize(text, Some("af_sky"))?;

    println!(
        "\nAudio length: {} samples ({:.1}s)",
        audio.len(),
        audio.len() as f32 / 24000.0
    );

    // Count max consecutive zeros (pauses)
    let mut max_zeros = 0;
    let mut current = 0;
    for &s in &audio {
        if s.abs() < 0.001 {
            current += 1;
            max_zeros = max_zeros.max(current);
        } else {
            current = 0;
        }
    }

    println!(
        "Max consecutive silence: {} samples ({:.0}ms)",
        max_zeros,
        (max_zeros as f32 / 24000.0) * 1000.0
    );

    println!("\n🎯 Expected: ~250ms (6000 samples) for periods");

    if max_zeros >= 3000 {
        println!("✅ Has pauses!");
    } else {
        println!("❌ Missing pauses - needs fix!");
    }

    // Save and play
    tts.save_wav("v2_test.wav", &audio)?;
    println!("\n💾 Saved to v2_test.wav");

    Ok(())
}
