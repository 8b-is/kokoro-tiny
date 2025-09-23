//! Baby speech example for mem8 integration
//! Shows how a baby AI learns to speak progressively

use kokoro_tiny::BabyTts;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("👶 Baby TTS for mem8 - Learning to speak!");
    println!("==========================================\n");

    // Initialize baby TTS
    let mut baby = BabyTts::new().await?;

    // Stage 1: Babbling (early development)
    println!("Stage 1: Babbling");
    let babble_audio = baby.babble()?;
    println!("  Generated babble with {} samples\n", babble_audio.len());

    // Stage 2: Single words
    println!("Stage 2: Single words");
    let words = ["mama", "dada", "milk", "up", "no"];
    for word in &words {
        println!("  Speaking: '{}'", word);
        let audio = baby.speak(word)?;
        println!("    ({} samples)", audio.len());
    }
    println!();

    // Stage 3: Two-word phrases
    println!("Stage 3: Two-word phrases");
    baby.grow();  // Increase vocabulary capacity
    let phrases = ["want milk", "up please", "bye bye"];
    for phrase in &phrases {
        println!("  Speaking: '{}'", phrase);
        let audio = baby.speak(phrase)?;
        println!("    ({} samples)", audio.len());
    }
    println!();

    // Stage 4: Simple sentences
    println!("Stage 4: Simple sentences");
    baby.grow();  // Further growth
    baby.grow();
    let sentence = "I want milk please";
    println!("  Speaking: '{}'", sentence);
    let audio = baby.speak(sentence)?;
    println!("    ({} samples)", audio.len());

    // Echo mode - learning from input
    println!("\nEcho mode (learning):");
    let echo_text = "hello mama";
    println!("  Echoing: '{}'", echo_text);
    let echo_audio = baby.echo(echo_text)?;
    println!("    Echo generated {} samples", echo_audio.len());

    // Simulate learning from audio
    baby.learn_from_audio(&echo_audio, echo_text)?;

    // Get audio parameters for mem8 integration
    let (sample_rate, channels, bits) = baby.get_audio_params();
    println!("\n📊 Audio format for mem8:");
    println!("  Sample rate: {}Hz", sample_rate);
    println!("  Channels: {} (mono)", channels);
    println!("  Bit depth: {}", bits);

    Ok(())
}