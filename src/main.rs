//! kokoro-speak: CLI tool for TTS announcements and alerts
//! Perfect for smart-tree integration and system notifications!

use clap::{Parser, Subcommand};
use kokoro_tiny::commit_tts::{
    self, CommitSummary, CommitTtsBackend,
};
use kokoro_tiny::TtsEngine;
use std::io::{self, BufRead};
use std::path::PathBuf;

#[derive(serde::Deserialize)]
struct SpeechMessage {
    voice: String,
    text: String,
    #[allow(dead_code)]
    priority: String,
}

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
    #[arg(short = 's', long, default_value = "1.0")]
    speed: f32,

    /// Audio gain/amplification (0.5 = quieter, 1.0 = normal, 2.0+ = louder, 4.0+ = maximum)
    #[arg(short = 'g', long, default_value = "1.5")]
    gain: f32,

    /// Language code ("auto", "en", "hu" / "magyar")
    #[arg(short = 'L', long, default_value = "auto")]
    lang: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Speak text directly
    Say {
        /// Text to speak
        text: String,
    },

    /// Speak Hungarian (Magyar) text directly with Hungarian IPA & voice preset
    Hu {
        /// Hungarian text to speak
        text: String,
    },

    /// Transcribe a WAV file to text & IPA (Speech-to-Text, defaults to Hungarian `hu`)
    Stt {
        /// Path to the WAV audio file
        wav_path: String,

        /// Language code ("hu", "en", "auto")
        #[arg(short, long, default_value = "hu")]
        lang: String,

        /// Emit JSON output
        #[arg(long)]
        json: bool,
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

    /// Hot Tub server for MEM8 integration
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value_t = 8420)]
        port: u16,
    },

    /// Speak a Git commit summary using our own TTS engines (Kokoro 82M / MEM|8 / LFM2.5-Audio)
    Commit {
        /// Path to the Git repository
        #[arg(short, long, default_value = ".")]
        repo: PathBuf,

        /// Commit revision (default: HEAD)
        #[arg(long, default_value = "HEAD")]
        rev: String,

        /// TTS backend engine (`kokoro`, `mem8`, `lfm`)
        #[arg(short, long, value_enum, default_value = "kokoro")]
        engine: CommitTtsBackend,

        /// Prepend the 3-trit (432/528/852 Hz) MEM|8 QuantTern harmonic pre-chime
        #[arg(long, default_value_t = true)]
        chime: bool,

        /// Daemon port to push to when `--async-fire` or `--peer` is set
        #[arg(short, long, default_value_t = 8420)]
        port: u16,

        /// Optional peer Mac host/IP (e.g. `MacBook.local` or `192.168.11.13`) to beam the commit summary to
        #[arg(long)]
        peer: Option<String>,

        /// Non-blocking fire-and-forget mode for `.git/hooks/post-commit` (< 2ms return)
        #[arg(long)]
        async_fire: bool,
    },

    /// Speak a `git pull` changelog summary (summarized to <= 60 seconds) using our own TTS engines
    Pull {
        /// Path to the Git repository
        #[arg(short, long, default_value = ".")]
        repo: PathBuf,

        /// Old revision before pull (default: `ORIG_HEAD`, fallback `HEAD~1`)
        #[arg(long)]
        old_rev: Option<String>,

        /// New revision after pull (default: `HEAD`)
        #[arg(long, default_value = "HEAD")]
        new_rev: String,

        /// TTS backend engine (`kokoro`, `mem8`, `lfm`)
        #[arg(short, long, value_enum, default_value = "kokoro")]
        engine: CommitTtsBackend,

        /// Prepend the 3-trit (432/528/852 Hz) MEM|8 QuantTern harmonic pre-chime
        #[arg(long, default_value_t = true)]
        chime: bool,

        /// Daemon port to push to when `--async-fire` or `--peer` is set
        #[arg(short, long, default_value_t = 8420)]
        port: u16,

        /// Optional peer Mac host/IP (e.g. `192.168.11.13`) to beam the pull summary to
        #[arg(long)]
        peer: Option<String>,

        /// Non-blocking fire-and-forget mode for `.git/hooks/post-merge` (< 2ms return)
        #[arg(long)]
        async_fire: bool,
    },

    /// Speak any GitHub commit summary (`owner/repo` or omit for the latest push across your GitHub orgs/accounts)
    Github {
        /// GitHub `owner/repo` (e.g. `8b-is/esp32.vaked.dev` or `standardgalactic/laboratory`). Omit for latest global push!
        repo: Option<String>,

        /// Commit SHA or ref (default: latest push SHA or HEAD)
        #[arg(long)]
        rev: Option<String>,

        /// Optional branch name hint
        #[arg(long)]
        branch: Option<String>,

        /// TTS backend engine (`kokoro`, `mem8`, `lfm`)
        #[arg(short, long, value_enum, default_value = "kokoro")]
        engine: CommitTtsBackend,

        /// Prepend the 3-trit (432/528/852 Hz) MEM|8 QuantTern harmonic pre-chime
        #[arg(long, default_value_t = true)]
        chime: bool,

        /// Daemon port to push to when `--async-fire` or `--peer` is set
        #[arg(short, long, default_value_t = 8420)]
        port: u16,

        /// Optional peer Mac host/IP (e.g. `MacBook.local`) to beam the GitHub commit summary to
        #[arg(long)]
        peer: Option<String>,

        /// Non-blocking fire-and-forget mode (pushes to running daemon on `:8420`)
        #[arg(long)]
        async_fire: bool,
    },

    /// Watch all Git repositories in a workspace + live GitHub PushEvents and speak every background commit automatically
    WatchCommits {
        /// Workspace root containing Git repositories
        #[arg(
            short,
            long,
            default_value = "/Users/lodripeter/workspace/peterlodri-sec"
        )]
        workspace: PathBuf,

        /// TCP port for instant `post-commit` hook events, peer Mac triggers & MEM|8 Hot Tub messages
        #[arg(short, long, default_value_t = 8420)]
        port: u16,

        /// TTS backend engine (`kokoro`, `mem8`, `lfm`)
        #[arg(short, long, value_enum, default_value = "kokoro")]
        engine: CommitTtsBackend,

        /// Prepend the 3-trit (432/528/852 Hz) MEM|8 QuantTern harmonic pre-chime
        #[arg(long, default_value_t = true)]
        chime: bool,
    },

    /// Install non-blocking `.git/hooks/post-commit` + global `~/.config/git/hooks/post-commit` TTS triggers
    InstallHooks {
        /// Workspace root containing Git repositories
        #[arg(
            short,
            long,
            default_value = "/Users/lodripeter/workspace/peterlodri-sec"
        )]
        workspace: PathBuf,

        /// Daemon port for the hook to push to
        #[arg(short, long, default_value_t = 8420)]
        port: u16,

        /// Also install global `~/.config/git/hooks/post-commit` (`core.hooksPath`) for ANY git repo on the machine
        #[arg(long, default_value_t = true)]
        global: bool,
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
            AlertType::Success => "af_bella", // Cheerful
            AlertType::Error => "am_adam",    // Serious male
            AlertType::Warning => "bf_emma",  // Clear British
            AlertType::Info => "af_sky",      // Default friendly
            AlertType::Build => "am_michael", // Professional
            AlertType::Test => "af_nicole",   // Precise
            AlertType::Deploy => "am_echo",   // Confident
            AlertType::Custom => "af_heart",  // Warm
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup tokio runtime for async operations
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let cli = Cli::parse();

    // Fast-path commands that must return in < 5ms without loading the 325 MB ONNX model:
    if let Some(Commands::InstallHooks {
        workspace,
        port,
        global,
    }) = &cli.command
    {
        let exe = std::env::current_exe()?;
        let count = commit_tts::install_workspace_hooks(workspace, &exe, *port)?;
        println!(
            "✅ Installed non-blocking commit & pull TTS hooks (post-commit + post-merge) across {} repositories in {}",
            count,
            workspace.display()
        );
        if *global {
            let global_path = commit_tts::install_global_git_hook(&exe, *port)?;
            println!(
                "🌍 Installed global Git post-commit & post-merge hooks at {} (triggers on ANY git repo on this Mac!)",
                global_path.display()
            );
        }
        return Ok(());
    }

    if let Some(Commands::Commit {
        repo,
        rev,
        port,
        peer,
        async_fire,
        ..
    }) = &cli.command
    {
        if let Some(peer_host) = peer {
            let summary = CommitSummary::from_repo(repo, rev)?;
            if commit_tts::try_push_summary_to_host(&summary, peer_host, *port) {
                println!(
                    "📡 Beamed commit [{} · {}] to peer {}:{}",
                    summary.repo_name, summary.short_sha, peer_host, port
                );
            }
        }
        if *async_fire {
            if !commit_tts::try_push_to_daemon(repo, rev, *port) {
                let exe = std::env::current_exe()?;
                commit_tts::spawn_detached_commit_speech(&exe, repo, rev)?;
            }
            return Ok(());
        }
    }

    if let Some(Commands::Pull {
        repo,
        old_rev,
        new_rev,
        port,
        peer,
        async_fire,
        ..
    }) = &cli.command
    {
        if let Some(peer_host) = peer {
            let summary = CommitSummary::from_pull(repo, old_rev.as_deref(), new_rev)?;
            if commit_tts::try_push_summary_to_host(&summary, peer_host, *port) {
                println!(
                    "📡 Beamed pull [{} · {}] to peer {}:{}",
                    summary.repo_name, summary.short_sha, peer_host, port
                );
            }
        }
        if *async_fire {
            if !commit_tts::try_push_pull_to_daemon(repo, old_rev.as_deref(), new_rev, *port) {
                let exe = std::env::current_exe()?;
                commit_tts::spawn_detached_pull_speech(&exe, repo, old_rev.as_deref(), new_rev)?;
            }
            return Ok(());
        }
    }

    if let Some(Commands::Github {
        repo,
        rev,
        branch,
        port,
        peer,
        async_fire: true,
        ..
    }) = &cli.command
    {
        if let Some(peer_host) = peer {
            let summary = CommitSummary::from_github(
                repo.as_deref(),
                rev.as_deref(),
                branch.as_deref(),
            )?;
            let _ = commit_tts::try_push_summary_to_host(&summary, peer_host, *port);
        }
        let _ = commit_tts::try_push_github_to_daemon(
            repo.as_deref(),
            rev.as_deref(),
            branch.as_deref(),
            *port,
        );
        return Ok(());
    }

    if let Some(Commands::Stt {
        wav_path,
        lang,
        json,
    }) = &cli.command
    {
        let stt = kokoro_tiny::SttEngine::new();
        let opts = kokoro_tiny::SttOptions::default().lang(lang);
        let result = stt.transcribe_wav(wav_path, Some(opts))?;
        if *json {
            println!("{}", serde_json::to_string_pretty(&result)?);
        } else {
            println!("🎧 STT Transcription [{} · {}]:", result.lang, result.backend);
            println!("   Text:       {}", result.text);
            if !result.ipa.is_empty() {
                println!("   IPA:        /{}/", result.ipa);
            }
            println!(
                "   Metrics:    {} ms · confidence {:.0}% · RMS {:.4} · voiced {:.0}% ({} syllables)",
                result.duration_ms,
                result.confidence * 100.0,
                result.rms_energy,
                result.voiced_ratio * 100.0,
                result.estimated_syllables
            );
        }
        return Ok(());
    }

    // Initialize TTS engine (uses ~/.cache/k automatically)
    let mut engine = rt
        .block_on(TtsEngine::new())
        .map_err(|e| format!("Failed to initialize TTS: {}", e))?;

    // List voices if requested
    if cli.list_voices {
        println!("🎤 Available voices:");
        for voice in engine.voices() {
            println!("  • {}", voice);
        }
        println!("\n🇭🇺 Hungarian (Magyar) voice presets:");
        for &(alias, blend, desc) in kokoro_tiny::hungarian::HUNGARIAN_VOICE_PRESETS {
            println!("  • {} -> {} ({})", alias, blend, desc);
        }
        return Ok(());
    }

    if let Some(Commands::Commit {
        repo,
        rev,
        engine: backend,
        chime,
        ..
    }) = &cli.command
    {
        let summary = CommitSummary::from_repo(repo, rev)?;
        commit_tts::speak_commit(
            &mut engine,
            &summary,
            *backend,
            cli.speed,
            cli.gain,
            cli.volume,
            cli.duck,
            cli.duck_level,
            *chime,
        )?;
        return Ok(());
    }

    if let Some(Commands::Pull {
        repo,
        old_rev,
        new_rev,
        engine: backend,
        chime,
        ..
    }) = &cli.command
    {
        let summary = CommitSummary::from_pull(repo, old_rev.as_deref(), new_rev)?;
        commit_tts::speak_commit(
            &mut engine,
            &summary,
            *backend,
            cli.speed,
            cli.gain,
            cli.volume,
            cli.duck,
            cli.duck_level,
            *chime,
        )?;
        return Ok(());
    }

    if let Some(Commands::Github {
        repo,
        rev,
        branch,
        engine: backend,
        chime,
        port,
        peer,
        ..
    }) = &cli.command
    {
        let summary = CommitSummary::from_github(
            repo.as_deref(),
            rev.as_deref(),
            branch.as_deref(),
        )?;
        if let Some(peer_host) = peer {
            if commit_tts::try_push_summary_to_host(&summary, peer_host, *port) {
                println!(
                    "📡 Also beamed GitHub commit [{} · {}] to peer {}:{}",
                    summary.repo_name, summary.short_sha, peer_host, port
                );
            }
        }
        commit_tts::speak_commit(
            &mut engine,
            &summary,
            *backend,
            cli.speed,
            cli.gain,
            cli.volume,
            cli.duck,
            cli.duck_level,
            *chime,
        )?;
        return Ok(());
    }

    if let Some(Commands::WatchCommits {
        workspace,
        port,
        engine: backend,
        chime,
    }) = &cli.command
    {
        commit_tts::run_commit_watcher_daemon(
            &mut engine,
            workspace,
            *port,
            *backend,
            cli.speed,
            cli.gain,
            cli.volume,
            cli.duck,
            cli.duck_level,
            *chime,
        )?;
        return Ok(());
    }

    if let Some(Commands::Serve { port }) = cli.command {
        rt.block_on(async {
            let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await.unwrap();
            println!("✅ Listening on TCP 127.0.0.1:{}", port);
            while let Ok((mut socket, addr)) = listener.accept().await {
                println!("📡 Accepted connection from {}", addr);
                use tokio::io::AsyncBufReadExt;
                let mut reader = tokio::io::BufReader::new(&mut socket);
                let mut line = String::new();
                if let Ok(n) = reader.read_line(&mut line).await {
                    if n > 0 {
                        if let Ok(msg) = serde_json::from_str::<SpeechMessage>(&line) {
                            println!("🗣️  Request [{}]: \"{}\"", msg.voice, msg.text);
                            // Map voice
                            let target_voice = if msg.voice.to_lowercase() == "aye" {
                                "af_sky"
                            } else {
                                &msg.voice
                            };
                            
                            let start = std::time::Instant::now();
                            match engine.synthesize_with_options(&msg.text, Some(target_voice), cli.speed, cli.gain, Some("en")) {
                                Ok(audio) => {
                                    println!("[INFO] Synthesis complete in {:?}", start.elapsed());
                                    let temp_path = "/tmp/mem8_kokoro_out.wav";
                                    if let Err(e) = engine.save_wav(temp_path, &audio) {
                                        eprintln!("⚠️ Failed to save audio: {}", e);
                                    } else {
                                        println!("✅ Synthesized successfully. Playing via ffplay...");
                                        let _ = std::process::Command::new("ffplay")
                                            .args(["-nodisp", "-autoexit", "-loglevel", "quiet", temp_path])
                                            .status();
                                    }
                                }
                                Err(e) => {
                                    eprintln!("⚠️ Synthesis error: {}", e);
                                }
                            }
                        } else {
                            eprintln!("⚠️ Failed to parse JSON payload");
                        }
                    }
                }
                println!("🔌 Disconnected {}", addr);
            }
        });
        return Ok(());
    }

    // Get text to speak based on command
    let (text, voice, resolved_lang) = match cli.command {
        Some(Commands::Say { text }) => (text, cli.voice, cli.lang.clone()),

        Some(Commands::Hu { text }) => {
            let v = if cli.voice == "af_sky" {
                "hu_mariska".to_string()
            } else {
                cli.voice
            };
            (text, v, "hu".to_string())
        }

        Some(Commands::Pipe) => {
            // Read from stdin
            let stdin = io::stdin();
            let mut lines = Vec::new();
            for line in stdin.lock().lines() {
                lines.push(line?);
            }
            (lines.join(" "), cli.voice, cli.lang.clone())
        }

        Some(Commands::Alert {
            alert_type,
            message,
        }) => {
            let text = message.unwrap_or_else(|| alert_type.default_message().to_string());
            let voice = alert_type.voice().to_string();
            (text, voice, cli.lang.clone())
        }

        Some(Commands::Context { text, prefix }) => {
            let full_text = format!("{} {}", prefix, text);
            // Use a clear, professional voice for context summaries
            (full_text, "bf_isabella".to_string(), cli.lang.clone())
        }

        Some(Commands::Serve { .. })
        | Some(Commands::Stt { .. })
        | Some(Commands::Commit { .. })
        | Some(Commands::Pull { .. })
        | Some(Commands::Github { .. })
        | Some(Commands::WatchCommits { .. })
        | Some(Commands::InstallHooks { .. }) => unreachable!(),

        None => {
            // Default: read from stdin if available, otherwise show help
            if atty::is(atty::Stream::Stdin) {
                eprintln!("💡 No input provided. Use --help for usage information.");
                eprintln!("\nQuick examples:");
                eprintln!("  kokoro-speak say \"Hello world!\"");
                eprintln!("  kokoro-speak hu \"Köszönöm szépen a figyelmet, Péter!\"");
                eprintln!("  kokoro-speak stt /tmp/magyar.wav --lang hu");
                eprintln!("  echo \"Build complete\" | kokoro-speak pipe");
                eprintln!("  kokoro-speak alert success");
                eprintln!(
                    "  kokoro-speak context \"Found 5 TypeScript files with 200 lines total\""
                );
                return Ok(());
            }

            let stdin = io::stdin();
            let mut lines = Vec::new();
            for line in stdin.lock().lines() {
                lines.push(line?);
            }
            (lines.join(" "), cli.voice, cli.lang.clone())
        }
    };

    // Synthesize speech with speed, gain, and language control
    let final_lang = kokoro_tiny::hungarian::normalize_lang_code(Some(&resolved_lang), &text);
    let audio = engine
        .synthesize_with_options(&text, Some(&voice), cli.speed, cli.gain, Some(&final_lang))
        .map_err(|e| format!("Synthesis failed: {}", e))?;

    // Output to file or play
    if let Some(output_path) = cli.output {
        engine
            .save_wav(&output_path, &audio)
            .map_err(|e| format!("Failed to save audio: {}", e))?;
        let ipa = if final_lang == "hu" {
            kokoro_tiny::hungarian::hungarian_graphemes_to_ipa(&text)
        } else {
            String::new()
        };
        kokoro_tiny::SttEngine::write_sidecar(&output_path, &text, &final_lang, &ipa);
        println!("💾 Saved to: {} [lang: {}]", output_path, final_lang);
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

            let preview = if text.chars().count() > 50 {
                format!("{}...", text.chars().take(50).collect::<String>())
            } else {
                text.clone()
            };

            println!(
                "🔊 Speaking [{}]: \"{}\" [voice: {}, speed: {}x, volume: {}{}{}]",
                final_lang,
                preview,
                voice,
                cli.speed,
                cli.volume,
                gain_info,
                ducking_info
            );

            // Use ducking if enabled
            if cli.duck {
                engine
                    .play_with_ducking(&audio, cli.volume, true, cli.duck_level)
                    .map_err(|e| format!("Playback with ducking failed: {}", e))?;
            } else {
                engine
                    .play(&audio, cli.volume)
                    .map_err(|e| format!("Playback failed: {}", e))?;
            }
        }

        #[cfg(not(feature = "playback"))]
        {
            // Fallback: save to temp file
            let temp_file = "/tmp/kokoro_output.wav";
            engine
                .save_wav(temp_file, &audio)
                .map_err(|e| format!("Failed to save audio: {}", e))?;
            println!(
                "💾 Audio saved to: {} (playback feature not enabled)",
                temp_file
            );
        }
    }

    Ok(())
}
