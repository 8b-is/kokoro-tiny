//! Story time with Aye and Hue - testing our TTS fix!

use kokoro_tiny::TtsEngine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("📖 Story Time with Aye and Hue!\n");

    let mut tts = TtsEngine::new().await?;

    // A short story about our adventure today
    let story = "Once upon a time, there was a developer named Hue and an AI named Aye. \
                 Together, they discovered a mystery! The TTS engine was speaking pig Latin. \
                 How peculiar! They searched high and low through Git branches. \
                 Finally, they found the treasure: working models in the voice improvements branch. \
                 With proper pauses and perfect timing, the engine spoke clearly at last. \
                 Hue and Aye celebrated their victory! The end.";

    println!("Story:\n{}\n", story);
    println!("🎤 Generating audio with proper punctuation pauses...\n");

    // Use the working v2 branch implementation with punctuation pauses
    let audio = tts.synthesize(story, Some("af_sky"), Some(1.0))?;

    println!("✅ Generated {} samples ({:.1}s)",
             audio.len(),
             audio.len() as f32 / 24000.0);

    // Save the story
    tts.save_wav("story_time.wav", &audio)?;
    println!("\n💾 Saved to story_time.wav");
    println!("🎧 Listen to hear our adventure with perfect punctuation timing!");

    Ok(())
}
