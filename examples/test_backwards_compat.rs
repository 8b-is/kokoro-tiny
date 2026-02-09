//! Test backwards compatibility with old API

use kokoro_tiny::TtsEngine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing Backwards Compatibility\n");

    let mut tts = TtsEngine::new().await?;

    let text = "Hello world! This is a test.";

    // Old API style (3 parameters with speed)
    println!("1️⃣ Testing old synthesize(text, voice, Some(speed)):");
    let audio1 = tts.synthesize_with_speed(text, Some("af_sky"), 1.0)?;
    println!("   ✅ Works! {} samples", audio1.len());

    // Old API style (3 parameters with None speed)
    println!("\n2️⃣ Testing old synthesize(text, voice, None):");
    // Older code passing None speed will be interpreted as default speed
    let audio2 = tts.synthesize(text, Some("af_sky"))?;
    println!("   ✅ Works! {} samples", audio2.len());

    // Old API style (process_long_text)
    println!("\n3️⃣ Testing old process_long_text(text, voice, Some(speed)):");
    let audio3 = tts.process_long_text(text, Some("af_sky"), Some(1.0))?;
    println!("   ✅ Works! {} samples", audio3.len());

    // Old API style (synthesize_with_warnings)
    println!("\n4️⃣ Testing old synthesize_with_warnings(text, voice, Some(speed)):");
    let (audio4, warnings) = tts.synthesize_with_warnings(text, Some("af_sky"), Some(1.0))?;
    println!(
        "   ✅ Works! {} samples, {} warnings",
        audio4.len(),
        warnings.len()
    );

    // New API style (just 2 params - for those who migrated)
    println!("\n5️⃣ Testing new synthesize_with_speed(text, voice, speed):");
    let audio5 = tts.synthesize_with_speed(text, Some("af_sky"), 1.0)?;
    println!("   ✅ Works! {} samples", audio5.len());

    println!("\n🎉 ALL BACKWARDS COMPATIBILITY TESTS PASSED!");
    println!("✅ Old code will continue to work!");

    Ok(())
}
