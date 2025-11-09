use kokoro_tiny::TtsEngine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut tts = TtsEngine::new().await?;
    let text = "It's 21:22";
    println!("Synthesizing: {}", text);
    let audio = tts.synthesize(text, None, None)?;
    tts.save_wav("output_short.wav", &audio)?;
    println!("Saved output_short.wav");
    Ok(())
}
