//! kokoro-speak: CLI tool for TTS announcements and alerts
//! Perfect for smart-tree integration and system notifications!

use clap::{Parser, Subcommand};
use kokoro_tiny::TtsEngine;
use std::io::{self, BufRead};

#[derive(Parser)]
#[command(name = "kokoro-speak")]
#[command(about = "🎤 Minimal TTS for alerts, logs, and announcements", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Volume level (0.0 to 1.0)
    #[arg(short, long, default_value = "0.8")]
    volume: f32,

    /// Voice to use
    #[arg(short = 'V', long, default_value = "af_sky")]
    voice: String,

    /// Save to file instead of playing
    #[arg(short, long)]
    output: Option<String>,

    /// List available voices
    #[arg(short, long)]
    list_voices: bool,

    /// Enable audio ducking - reduces other audio while speaking
    #[arg(short = 'd', long)]
    duck: bool,

    /// Duck level - how much to reduce other audio (0.0 = mute, 1.0 = no change)
    #[arg(long, default_value = "0.3")]
    duck_level: f32,

    /// Speech speed (0.5 = slower, 1.0 = normal, 2.0 = faster)
    #[arg(short = 's', long, default_value = "0.85")]
    speed: f32,

    /// Audio gain/amplification (0.5 = quieter, 1.0 = normal, 2.0+ = louder, 4.0+ = maximum)
    #[arg(short = 'g', long, default_value = "1.5")]
    gain: f32,
}

#[derive(Subcommand)]
enum Commands {
    /// Speak text directly
    Say {
        /// Text to speak
        text: String,
    },

    /// Read from stdin (perfect for piped input)
    Pipe,

    /// System alert with preset messages
    Alert {
        #[arg(value_enum)]
        alert_type: AlertType,

        /// Optional custom message
        message: Option<String>,
    },

    /// Context summary mode for smart-tree
    Context {
        /// Summary text
        text: String,

        /// Prefix for context (e.g., "Smart tree context update:")
        #[arg(short, long, default_value = "Context summary:")]
        prefix: String,
    },
}

#[derive(clap::ValueEnum, Clone)]
enum AlertType {
    Success,
    Error,
    Warning,
    Info,
    Build,
    Test,
    Deploy,
    Custom,
}

impl AlertType {
    fn default_message(&self) -> &str {
        match self {
            AlertType::Success => "Operation completed successfully!",
            AlertType::Error => "Error detected. Please check the logs.",
            AlertType::Warning => "Warning: Attention required.",
            AlertType::Info => "Information update available.",
            AlertType::Build => "Build process complete.",
            AlertType::Test => "Test suite finished running.",
            AlertType::Deploy => "Deployment status update.",
            AlertType::Custom => "Alert triggered.",
        }
    }

    fn voice(&self) -> &str {
        match self {
            AlertType::Success => "af_bella",  // Cheerful
            AlertType::Error => "am_adam",     // Serious male
            AlertType::Warning => "bf_emma",   // Clear British
            AlertType::Info => "af_sky",       // Default friendly
            AlertType::Build => "am_michael",  // Professional
            AlertType::Test => "af_nicole",    // Precise
            AlertType::Deploy => "am_echo",    // Confident
            AlertType::Custom => "af_heart",   // Warm
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup tokio runtime for async operations
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let cli = Cli::parse();

    // Initialize TTS engine (uses ~/.cache/k automatically)
    let mut engine = rt.block_on(TtsEngine::new())
        .map_err(|e| format!("Failed to initialize TTS: {}", e))?;

    // List voices if requested
    if cli.list_voices {
        println!("🎤 Available voices:");
        for voice in engine.voices() {
            println!("  • {}", voice);
        }
        return Ok(());
    }

    // Get text to speak based on command
    let (text, voice) = match cli.command {
        Some(Commands::Say { text }) => {
            (text, cli.voice)
        }

        Some(Commands::Pipe) => {
            // Read from stdin
            let stdin = io::stdin();
            let mut lines = Vec::new();
            for line in stdin.lock().lines() {
                lines.push(line?);
            }
            (lines.join(" "), cli.voice)
        }

        Some(Commands::Alert { alert_type, message }) => {
            let text = message.unwrap_or_else(|| alert_type.default_message().to_string());
            let voice = alert_type.voice().to_string();
            (text, voice)
        }

        Some(Commands::Context { text, prefix }) => {
            let full_text = format!("{} {}", prefix, text);
            // Use a clear, professional voice for context summaries
            (full_text, "bf_isabella".to_string())
        }

        None => {
            // Default: read from stdin if available, otherwise show help
            if atty::is(atty::Stream::Stdin) {
                eprintln!("💡 No input provided. Use --help for usage information.");
                eprintln!("\nQuick examples:");
                eprintln!("  kokoro-speak say \"Hello world!\"");
                eprintln!("  echo \"Build complete\" | kokoro-speak pipe");
                eprintln!("  kokoro-speak alert success");
                eprintln!("  kokoro-speak context \"Found 5 TypeScript files with 200 lines total\"");
                return Ok(());
            }

            let stdin = io::stdin();
            let mut lines = Vec::new();
            for line in stdin.lock().lines() {
                lines.push(line?);
            }
            (lines.join(" "), cli.voice)
        }
    };

    // Synthesize speech with speed and gain control
    let audio = engine.synthesize_with_options(&text, Some(&voice), cli.speed, cli.gain)
        .map_err(|e| format!("Synthesis failed: {}", e))?;

    // Output to file or play
    if let Some(output_path) = cli.output {
        engine.save_wav(&output_path, &audio)
            .map_err(|e| format!("Failed to save audio: {}", e))?;
        println!("💾 Saved to: {}", output_path);
    } else {
        #[cfg(feature = "playback")]
        {
            let ducking_info = if cli.duck {
                format!(", ducking: {:.0}%", (1.0 - cli.duck_level) * 100.0)
            } else {
                String::new()
            };

            let gain_info = if cli.gain != 1.0 {
                format!(", gain: {}x", cli.gain)
            } else {
                String::new()
            };

            println!("🔊 Speaking: \"{}\" [voice: {}, speed: {}x, volume: {}{}{}]",
                if text.len() > 50 {
                    format!("{}...", &text[..50])
                } else {
                    text.clone()
                },
                voice,
                cli.speed,
                cli.volume,
                gain_info,
                ducking_info
            );

            // Use ducking if enabled
            if cli.duck {
                engine.play_with_ducking(&audio, cli.volume, true, cli.duck_level)
                    .map_err(|e| format!("Playback with ducking failed: {}", e))?;
            } else {
                engine.play(&audio, cli.volume)
                    .map_err(|e| format!("Playback failed: {}", e))?;
            }
        }

        #[cfg(not(feature = "playback"))]
        {
            // Fallback: save to temp file
            let temp_file = "/tmp/kokoro_output.wav";
            engine.save_wav(temp_file, &audio)
                .map_err(|e| format!("Failed to save audio: {}", e))?;
            println!("💾 Audio saved to: {} (playback feature not enabled)", temp_file);
        }
    }

    Ok(())
}