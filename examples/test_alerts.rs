//! Test various alert sizes and unicode handling
//!
//! This example demonstrates the improved speech output capabilities:
//! - Handles small alerts consistently
//! - Normalizes unicode characters properly
//! - Validates text and provides warnings
//! - Maintains consistent speed regardless of text length

use kokoro_tiny::TtsEngine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎤 Testing Alert Speech Output Improvements\n");
    println!("==========================================\n");

    // Initialize TTS engine
    let mut tts = TtsEngine::new().await
        .map_err(|e| format!("Failed to initialize TTS: {}", e))?;

    // Test cases demonstrating various improvements
    let test_cases = vec![
        // Small alerts (< 50 chars) - now use direct synthesis for consistency
        ("Error!", "Very short alert"),
        ("Success", "Single word"),
        ("Build complete.", "Short sentence"),

        // Unicode handling - smart quotes, dashes, ellipsis
        (""Hello world"", "Smart quotes"),
        ("Don't worry—it's fine", "Smart apostrophe and em dash"),
        ("Loading…", "Ellipsis character"),
        ("Price: $10–$20", "En dash"),

        // Edge cases
        ("  Multiple   spaces   ", "Multiple spaces (normalized)"),
        ("Line1\nLine2\nLine3", "Newlines (converted to spaces)"),
        ("\tTabbed\ttext", "Tabs (converted to spaces)"),

        // Normal length text (> 50 chars)
        ("This is a longer message that will be processed with sentence splitting.", "Normal length"),

        // Mixed punctuation
        ("Alert! Warning: Check logs. Details available.", "Mixed punctuation with pauses"),
    ];

    for (text, description) in test_cases {
        println!("📝 Test: {}", description);
        println!("   Input: {:?}", text);

        // Synthesize with warnings
        match tts.synthesize_with_warnings(text, Some("af_sky"), Some(1.0)) {
            Ok((audio, warnings)) => {
                println!("   ✅ Generated {} audio samples", audio.len());

                if !warnings.is_empty() {
                    println!("   ⚠️  Warnings:");
                    for warning in warnings {
                        println!("       - {}", warning);
                    }
                }
            }
            Err(e) => {
                println!("   ❌ Error: {}", e);
            }
        }

        println!();
    }

    println!("==========================================");
    println!("\n✨ Key Improvements:");
    println!("  • Small alerts (< 50 chars) use direct synthesis for consistent speed");
    println!("  • Unicode normalization handles smart quotes, dashes, ellipsis");
    println!("  • Text validation provides warnings for potential issues");
    println!("  • Empty text handled gracefully");
    println!("  • Whitespace normalized (tabs, newlines, multiple spaces)");
    println!("  • Characters not in vocabulary are detected and reported");

    Ok(())
}
