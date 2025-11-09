//! Test example showing voice style selection based on token count
//!
//! This example demonstrates:
//! - How different text lengths result in different voice styles being selected
//! - How text exceeding the token limit is automatically split
//!
//! Run with: cargo run --example test_voice_styles

use kokoro_tiny::TtsEngine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎤 Testing Voice Style Selection Based on Token Count\n");

    // Initialize the TTS engine
    let mut tts = TtsEngine::new().await?;

    // Test 1: Short text (few tokens)
    println!("Test 1: Short text");
    let short_text = "Hello!";
    let audio1 = tts.synthesize(short_text, Some("af_sky"), None)?;
    println!("  Text: \"{}\"", short_text);
    println!("  Generated {} audio samples", audio1.len());
    tts.save_wav("test_short.wav", &audio1)?;
    println!("  Saved to test_short.wav\n");

    // Test 2: Medium text (more tokens)
    println!("Test 2: Medium text");
    let medium_text = "This is a medium length sentence that contains several words and should produce a different voice style based on the number of tokens.";
    let audio2 = tts.synthesize(medium_text, Some("af_sky"), None)?;
    println!("  Text: \"{}...\"", &medium_text[..50]);
    println!("  Generated {} audio samples", audio2.len());
    tts.save_wav("test_medium.wav", &audio2)?;
    println!("  Saved to test_medium.wav\n");

    // Test 3: Long text (many tokens)
    println!("Test 3: Long text");
    let long_text = "This is a much longer piece of text that contains many more words and sentences. \
                     It should demonstrate how the voice style selection adapts to longer content. \
                     The text-to-speech engine will automatically select the appropriate voice style \
                     based on the number of tokens in the input. This helps produce more natural \
                     sounding speech that doesn't have clipped words or quiet segments.";
    let audio3 = tts.synthesize(long_text, Some("af_sky"), None)?;
    println!("  Text: \"{}...\"", &long_text[..50]);
    println!("  Generated {} audio samples", audio3.len());
    tts.save_wav("test_long.wav", &audio3)?;
    println!("  Saved to test_long.wav\n");

    // Test 4: Very long text (testing token limit handling)
    println!("Test 4: Very long text (testing automatic chunking)");
    let very_long_text = format!(
        "{}. {}. {}. {}.",
        long_text, long_text, long_text, long_text
    );
    let audio4 = tts.synthesize(&very_long_text, Some("af_sky"), None)?;
    println!("  Text length: {} characters", very_long_text.len());
    println!("  Generated {} audio samples", audio4.len());
    tts.save_wav("test_very_long.wav", &audio4)?;
    println!("  Saved to test_very_long.wav\n");

    println!("✅ All tests completed!");
    println!("\nThe audio files demonstrate how voice style selection improves speech quality:");
    println!("- Short text uses style optimized for few tokens");
    println!("- Medium text uses style optimized for moderate token count");
    println!("- Long text uses style optimized for many tokens");
    println!("- Very long text is automatically split to respect token limits");

    Ok(())
}
