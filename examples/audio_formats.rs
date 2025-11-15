//! Test all audio format outputs - WAV, MP3, OPUS, FLAC
//! Shows file sizes and characteristics for comparison

use kokoro_tiny::TtsEngine;
use std::fs;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎵 Testing all audio formats with kokoro-tiny!");
    println!("================================================\n");

    // Initialize TTS engine
    let mut tts = TtsEngine::new().await?;

    // Test messages
    let test_cases = vec![
        ("short", "Hello world!", "af_sky"),
        ("medium", "Testing audio formats. OPUS for streaming, FLAC for quality, MP3 for compatibility.", "bf_isabella"),
        ("alert", "Build complete! All tests passed successfully.", "am_michael"),
    ];

    for (label, text, voice) in test_cases {
        println!("📝 Test: {} - \"{}\"", label, text);
        println!("🎤 Voice: {}\n", voice);

        // Generate audio
        let start = Instant::now();
        let audio = tts.synthesize(text, Some(voice), None)?;
        let synthesis_time = start.elapsed();

        println!("⚡ Synthesis: {:.2}ms ({} samples)",
            synthesis_time.as_millis(),
            audio.len()
        );

        // Test each format
        let formats = vec![
            ("wav", format!("{}_test.wav", label)),
            ("mp3", format!("{}_test.mp3", label)),
            ("opus", format!("{}_test.opus", label)),
            ("flac", format!("{}_test.flac", label)),
        ];

        println!("\n📊 Format comparison:");
        println!("  Format | File Size  | Save Time | Notes");
        println!("  -------|------------|-----------|-------");

        for (format, filename) in &formats {
            let start = Instant::now();

            let result = match *format {
                "wav" => tts.save_wav(filename.as_str(), &audio),

                #[cfg(feature = "mp3")]
                "mp3" => tts.save_mp3(&filename, &audio),
                #[cfg(not(feature = "mp3"))]
                "mp3" => {
                    println!("  MP3    | [disabled] |           | Enable with --features mp3");
                    continue;
                }

                #[cfg(feature = "opus-format")]
                "opus" => tts.save_opus(&filename, &audio, 24000),
                #[cfg(not(feature = "opus-format"))]
                "opus" => {
                    println!("  OPUS   | [disabled] |           | Enable with --features opus-format");
                    continue;
                }

                #[cfg(feature = "flac-format")]
                "flac" => tts.save_flac(&filename, &audio),
                #[cfg(not(feature = "flac-format"))]
                "flac" => {
                    println!("  FLAC   | [disabled] |           | Enable with --features flac-format");
                    continue;
                }

                _ => continue,
            };

            let save_time = start.elapsed();

            if let Err(e) = result {
                println!("  {:6} | ERROR      | {:.2}ms    | {}",
                    format.to_uppercase(),
                    save_time.as_millis(),
                    e
                );
                continue;
            }

            // Get file size
            let metadata = fs::metadata(&filename)?;
            let size_kb = metadata.len() as f64 / 1024.0;

            let notes = match *format {
                "wav" => "Uncompressed, best compatibility",
                "mp3" => "Lossy, universal support",
                "opus" => "Best for streaming/VoIP",
                "flac" => "Lossless compression, audiophile",
                _ => ""
            };

            println!("  {:6} | {:8.1}KB | {:7.2}ms | {}",
                format.to_uppercase(),
                size_kb,
                save_time.as_millis(),
                notes
            );
        }

        println!("\n----------------------------------------\n");
    }

    // Multi-format save test using save_audio()
    println!("🔄 Testing automatic format detection:");
    let test_audio = tts.synthesize("Format auto-detection test!", Some("af_heart"), None)?;

    let auto_files = vec![
        "auto_test.wav",
        "auto_test.mp3",
        "auto_test.opus",
        "auto_test.flac",
    ];

    for file in auto_files {
        match tts.save_audio(file, &test_audio) {
            Ok(_) => println!("  ✅ {}", file),
            Err(e) => println!("  ❌ {} - {}", file, e),
        }
    }

    println!("\n✨ All tests complete! Check the output files.");
    println!("\n💡 Tips:");
    println!("  • OPUS: Smallest files, great for real-time/streaming");
    println!("  • FLAC: Lossless quality, larger files");
    println!("  • MP3: Universal compatibility");
    println!("  • WAV: Uncompressed, fastest to save");

    Ok(())
}