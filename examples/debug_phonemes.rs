//! Debug phoneme generation

use espeak_rs::text_to_phonemes;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize espeak
    if let Err(e) = std::env::var("PIPER_ESPEAKNG_DATA_DIRECTORY") {
        println!("⚠️  PIPER_ESPEAKNG_DATA_DIRECTORY not set: {}", e);
        println!("Setting to: /opt/homebrew/Cellar/espeak-ng/1.52.0/share");
        std::env::set_var(
            "PIPER_ESPEAKNG_DATA_DIRECTORY",
            "/opt/homebrew/Cellar/espeak-ng/1.52.0/share",
        );
    }

    let text = "Hello world";

    println!("🔍 Testing phoneme generation for: \"{}\"\n", text);

    // Try different espeak settings
    println!("1️⃣ With punctuation preservation:");
    match text_to_phonemes(text, "en", None, true, false) {
        Ok(phonemes) => {
            println!("   Raw phonemes: {:?}", phonemes);
            println!("   Joined: {}", phonemes.join(" "));
        }
        Err(e) => println!("   Error: {:?}", e),
    }

    println!("\n2️⃣ Without punctuation preservation:");
    match text_to_phonemes(text, "en", None, false, false) {
        Ok(phonemes) => {
            println!("   Raw phonemes: {:?}", phonemes);
            println!("   Joined: {}", phonemes.join(" "));
        }
        Err(e) => println!("   Error: {:?}", e),
    }

    println!("\n3️⃣ With stress markers:");
    match text_to_phonemes(text, "en", None, true, true) {
        Ok(phonemes) => {
            println!("   Raw phonemes: {:?}", phonemes);
            println!("   Joined: {}", phonemes.join(" "));
        }
        Err(e) => println!("   Error: {:?}", e),
    }

    println!("\n4️⃣ Testing with en-us variant:");
    match text_to_phonemes(text, "en-us", None, true, false) {
        Ok(phonemes) => {
            println!("   Raw phonemes: {:?}", phonemes);
            println!("   Joined: {}", phonemes.join(" "));
        }
        Err(e) => println!("   Error: {:?}", e),
    }

    Ok(())
}
