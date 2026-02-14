# 🎤 kokoro-tiny

**Version 0.2.0** | Minimal, embeddable Text-to-Speech using the Kokoro 82M parameter model

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2021-orange.svg)](https://www.rust-lang.org)

> A minimal TTS crate perfect for embedding in other applications. Auto-downloads models on first run and caches them for instant reuse.

---

## ✨ Features

- 🚀 **Zero-config** - Auto-downloads and caches 82M Kokoro model (~310MB) and voices (~27MB)
- 🎵 **Multiple Voices** - 20+ voice presets including male, female, British, American accents
- 🎛️ **Voice Mixing** - Blend voices with weighted combinations (e.g., `"af_sky.4+af_nicole.5"`)
- ⚡ **Speed Control** - Adjustable speech speed (0.5x to 2.0x+)
- 🔊 **Audio Ducking** - Automatically reduces system volume during TTS playback
- 📦 **Multiple Formats** - WAV (built-in), MP3, and OPUS support
- 🎮 **Direct Playback** - Optional audio playback via rodio/cpal
- 🔄 **Streaming Mode** - Unlimited speech with interruption support
- 🧠 **MEM8 Integration** - Consciousness layer for AI memory persistence
- 📱 **CLI Tool** - `kokoro-speak` binary for alerts and announcements
- 🤖 **MCP Server** ⭐ NEW! - AI collaboration with voice (Claude can speak to you!)

---

## 🚀 Quick Start

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
kokoro-tiny = "0.2.0"
```

**Note**: The default build has NO system dependencies - it only generates WAV files.
For audio playback, enable the `playback` feature (requires ALSA on Linux):

```toml
[dependencies]
kokoro-tiny = { version = "0.2.0", features = ["playback"] }
```

Or build with playback enabled:
```bash
cargo build --features playback
```

For full functionality (playback, ducking, all formats):
```toml
[dependencies]
kokoro-tiny = { version = "0.2.0", features = ["full"] }
```

### System Requirements

**All Platforms (Required):**
```bash
# Debian/Ubuntu
sudo apt install espeak-ng

# Fedora/RHEL
sudo dnf install espeak-ng

# macOS
brew install espeak-ng

# Windows
# Download and install from: https://github.com/espeak-ng/espeak-ng/releases
```

**Optional - For Audio Playback Feature:**

If you enable the `playback` feature, additional system libraries are needed:

```bash
# Debian/Ubuntu - for playback feature only
sudo apt install libasound2-dev

# Fedora/RHEL - for playback feature only  
sudo dnf install alsa-lib-devel

# macOS - no additional deps needed
# Windows - no additional deps needed
```

**Note**: The default build requires NO audio system libraries - it generates WAV files only!

### Basic Usage

```rust
use kokoro_tiny::TtsEngine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize (downloads models on first run to ~/.cache/k/)
    let mut tts = TtsEngine::new().await?;

    // Generate speech with default voice (af_sky)
    let audio = tts.synthesize("Hello world!", None)?;

    // Save to WAV file
    tts.save_wav("output.wav", &audio)?;

    // Or play directly (requires 'playback' feature)
    #[cfg(feature = "playback")]
    tts.play(&audio, 0.8)?;

    Ok(())
}
```

### Advanced Synthesis Options

```rust
use kokoro_tiny::{TtsEngine, SynthesizeOptions};

let mut tts = TtsEngine::new().await?;

// Builder-style API for full control
let opts = SynthesizeOptions::default()
    .voice("af_bella")     // Choose voice
    .speed(1.2)            // 20% faster
    .gain(1.5);            // Louder output

let audio = tts.synthesize_with("Custom speech", opts)?;
```

---

## 🎙️ Voice Presets

kokoro-tiny includes 20+ built-in voices:

| Voice | Description | Use Case |
|-------|-------------|----------|
| `af_sky` | Friendly American female (default) | General purpose |
| `af_bella` | Cheerful American female | Success messages |
| `af_nicole` | Precise American female | Technical content |
| `af_heart` | Warm American female | Personal messages |
| `am_adam` | Serious American male | Error alerts |
| `am_michael` | Professional American male | Business/formal |
| `am_echo` | Confident American male | Announcements |
| `bf_emma` | Clear British female | Warnings |
| `bm_george` | British male | Narration |

**List all available voices:**
```bash
kokoro-speak --list-voices
```

**Voice Mixing:**
```rust
// 60% af_sky + 40% af_nicole
let audio = tts.synthesize("Blended voice", Some("af_sky.6+af_nicole.4"))?;
```

---

## 🛠️ Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `playback` | ❌ | Direct audio playback via rodio/cpal (requires ALSA on Linux) |
| `ducking` | ❌ | Audio ducking - reduces other audio during TTS |
| `mp3` | ❌ | MP3 encoding support |
| `opus-format` | ❌ | OPUS audio format |
| `cuda` | ❌ | CUDA acceleration for ONNX Runtime |
| `all-formats` | ❌ | Enables mp3 + opus-format |
| `full` | ❌ | Enables playback + ducking + all-formats |

**Default Build**: No features enabled - generates WAV files only, no system dependencies!

**Examples:**

```toml
# Default - WAV generation only (no system dependencies!)
kokoro-tiny = "0.2.0"

# With playback support (requires ALSA on Linux)
kokoro-tiny = { version = "0.2.0", features = ["playback"] }

# Minimal (no playback, just synthesis)
kokoro-tiny = { version = "0.2.0", default-features = false }

# All audio formats (no playback)
kokoro-tiny = { version = "0.2.0", features = ["all-formats"] }

# Full functionality (playback + ducking + all formats)
kokoro-tiny = { version = "0.2.0", features = ["full"] }

# CUDA acceleration
kokoro-tiny = { version = "0.2.0", features = ["cuda"] }
```

---

## 🤖 MCP Server for AI Collaboration ⭐ NEW!

kokoro-tiny now includes an MCP (Model Context Protocol) server that enables AI assistants like Claude to speak directly to you!

### What This Enables

**AI with Voice** - Claude can:
- 🔊 Get your attention with audio alerts
- 🎉 Celebrate successes with encouraging voice
- 📊 Provide status updates while you work
- ⚠️ Alert you to important findings
- 💬 Speak summaries and explanations

**Example Interaction:**
```
You: "Claude, help me debug this code"

Claude: *analyzes*
        🔊 "Hey! I found the issue in line 42!"
        
You: *looks up from other work*

Claude: *after you fix it*
        🔊 "Perfect! All tests passing. Incredible work!"
```

### Installation

**1. Build the MCP server:**
```bash
cargo install --path . --features playback --bin kokoro-mcp
```

**2. Add to Claude Desktop config:**

Edit `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS) or equivalent:
```json
{
  "mcpServers": {
    "kokoro-tts": {
      "command": "kokoro-mcp",
      "args": []
    }
  }
}
```

**3. Restart Claude Desktop**

Now Claude can use these tools:
- `speak_to_user` - Speak messages with specific voices
- `speak_with_emotion` - Auto-select voice by emotion
- `list_voices` - See available voices
- `synthesize_to_file` - Save audio to file

### Voice Emotions

Claude will automatically choose voices based on context:

| Emotion | Voice | Example Use |
|---------|-------|-------------|
| happy, success | af_bella | "All tests passed!" |
| alert, error | am_adam | "Build failed!" |
| info, friendly | af_sky | "Analysis complete." |
| technical | af_nicole | "Memory usage: 42%" |
| professional | am_michael | "Here's the solution..." |
| warm, encouraging | af_heart | "Great progress!" |

### Example MCP Tool Calls

**Speak to user:**
```javascript
await use_mcp_tool({
  server_name: "kokoro-tts",
  tool_name: "speak_to_user",
  arguments: {
    text: "Build completed successfully!",
    voice: "af_bella",
    speed: 1.0
  }
});
```

**Speak with emotion:**
```javascript
await use_mcp_tool({
  server_name: "kokoro-tts",
  tool_name: "speak_with_emotion",
  arguments: {
    text: "Warning: potential memory leak detected.",
    emotion: "alert"
  }
});
```

---

## 📦 CLI Tool: kokoro-speak

The included `kokoro-speak` binary provides command-line TTS:

### Installation

```bash
cargo install kokoro-tiny --features playback
```

### Usage Examples

**Basic text-to-speech:**
```bash
kokoro-speak say "Hello from Kokoro!"
```

**Pipe mode (read from stdin):**
```bash
echo "Processing complete" | kokoro-speak pipe
```

**Alert presets with automatic voice selection:**
```bash
kokoro-speak alert success "Build completed!"
kokoro-speak alert error "Tests failed"
kokoro-speak alert warning "Low disk space"
```

**Custom voice and speed:**
```bash
kokoro-speak -V af_bella -s 1.2 say "Fast and cheerful!"
```

**Audio ducking (reduces other audio):**
```bash
kokoro-speak --duck say "Important announcement"
```

**Save to file instead of playing:**
```bash
kokoro-speak -o output.wav say "Save me!"
```

---

## 🔧 Configuration & Caching

### Model Storage

Models are automatically downloaded to `~/.cache/k/`:
- `0.onnx` - Kokoro model (~310MB)
- `0.bin` - Voice embeddings (~27MB)
- `audio_device.txt` - Cached audio device preference

### Device Selection (playback feature)

When `playback` is enabled:

1. **Auto-selection**: Engine automatically picks a suitable audio device on first run
2. **Caching**: Selected device is saved to `~/.cache/k/audio_device.txt`
3. **Override**: Change at runtime:

```rust
// List available devices
let devices = tts.list_audio_devices()?;
for device in devices {
    println!("{}", device);
}

// Set specific device
tts.set_audio_device(Some("Built-in Output".to_string()))?;

// Clear preference (triggers auto-selection)
tts.set_audio_device(None)?;
```

The engine prefers devices with these keywords: `AirPods`, `Built-in`, `Voice`, `Headphones`, `Speaker`.

---

## 📚 API Reference

### Core Methods

```rust
impl TtsEngine {
    // Basic synthesis
    pub async fn new() -> Result<Self, String>;
    pub fn synthesize(&mut self, text: &str, voice: Option<&str>) -> Result<Vec<f32>, String>;
    pub fn synthesize_with_speed(&mut self, text: &str, voice: Option<&str>, speed: f32) -> Result<Vec<f32>, String>;
    pub fn synthesize_with(&mut self, text: &str, opts: SynthesizeOptions) -> Result<Vec<f32>, String>;
    
    // Audio output
    pub fn save_wav(&self, path: &str, audio: &[f32]) -> Result<(), String>;
    pub fn save_mp3(&self, path: &str, audio: &[f32]) -> Result<(), String>; // Requires 'mp3' feature
    pub fn save_opus(&self, path: &str, audio: &[f32], bitrate: i32) -> Result<(), String>; // Requires 'opus-format' feature
    pub fn to_wav_bytes(&self, audio: &[f32]) -> Result<Vec<u8>, String>;
    
    // Playback (requires 'playback' feature)
    pub fn play(&self, audio: &[f32], volume: f32) -> Result<(), String>;
    pub fn play_with_ducking(&self, audio: &[f32], volume: f32, duck_level: f32) -> Result<(), String>; // Requires 'ducking' feature
    
    // Device management
    pub fn list_audio_devices(&self) -> Result<Vec<String>, String>;
    pub fn set_audio_device(&mut self, device_name: Option<String>) -> Result<(), String>;
    pub fn get_audio_device(&self) -> Option<&str>;
    
    // Voice management
    pub fn voices(&self) -> Vec<String>;
}
```

### SynthesizeOptions Builder

```rust
let opts = SynthesizeOptions::default()
    .voice("af_sky")    // Voice preset or mix (e.g., "af_sky.6+af_bella.4")
    .speed(1.0)         // Speed multiplier (0.5-2.0+)
    .gain(1.5);         // Volume amplification (0.5-4.0+)
```

---

## 🎯 Examples

The repository includes several examples:

| Example | Description |
|---------|-------------|
| `simple.rs` | Basic TTS usage |
| `device_select.rs` | List and test audio devices |
| `unlimited_speech.rs` | Streaming mode with interruption |
| `mem8_baby.rs` | Baby speech from MEM8 consciousness |
| `all_voices.rs` | Demo all available voices |
| `story_time.rs` | Long-form content synthesis |

Run examples:
```bash
cargo run --example simple --features playback
cargo run --example device_select --features playback
```

---

## 🏗️ Building & Testing

### Build

```bash
# Debug build (no playback, no system dependencies)
cargo build

# Release build (recommended for performance)
cargo build --release

# With playback support (requires ALSA on Linux)
cargo build --release --features playback

# Full features (playback + ducking + all audio formats)
cargo build --release --features full
```

### Cross-Compilation

For cross-compiling (e.g., to ARM/aarch64), see the [ALSA cross-compilation guide](https://github.com/RustAudio/rodio#cross-compiling-aarchaarch64arm).

**Note**: Cross-compilation only applies if using the `playback` feature. Default builds have no system dependencies and cross-compile easily.

### Run Tests

```bash
cargo test
```

### Linting

```bash
# Format code
cargo fmt

# Run clippy
cargo clippy --all-targets --all-features -- -D warnings
```

### Management Script

Use the included `scripts/manage.sh` for easy project management:

```bash
./scripts/manage.sh             # Interactive mode
./scripts/manage.sh build       # Build project
./scripts/manage.sh test        # Run all checks
./scripts/manage.sh status      # Check system status
./scripts/manage.sh run "text"  # Quick TTS test
```

---

## 🔧 Advanced Features

### Why No Default Playback?

kokoro-tiny is designed to be maximally portable and embeddable:

- **Default build**: Zero system dependencies (except espeak-ng)
- **Perfect for**: Servers, embedded systems, CI/CD, containers
- **Core functionality**: TTS synthesis and WAV generation work everywhere

Audio playback requires platform-specific libraries:
- **Linux**: ALSA (`libasound2-dev` / `alsa-lib-devel`)
- **macOS**: CoreAudio (built-in)
- **Windows**: WASAPI (built-in)

By making playback optional:
- ✅ Library builds on any platform
- ✅ No build-time system dependencies by default
- ✅ Users opt-in to playback when needed
- ✅ Minimal attack surface for security-conscious deployments

**Enable playback when you need it:**
```toml
kokoro-tiny = { version = "0.2.0", features = ["playback"] }
```

### Audio Ducking

Automatically reduces other audio during TTS playback:

```rust
// Requires 'ducking' feature
tts.play_with_ducking(&audio, 0.8, 0.3)?; 
// volume=0.8, duck_level=0.3 (reduce other audio to 30%)
```

### Streaming Mode

For unlimited speech with interruption support:

```rust
use kokoro_tiny::streaming::StreamingSynthesizer;

let mut stream = StreamingSynthesizer::new(tts, "af_sky");
stream.start_streaming()?;

// Add text continuously
stream.add_text("Part 1. ");
stream.add_text("Part 2. ");

// Interrupt and replace
stream.interrupt_and_speak("Emergency message!")?;
```

### MEM8 Integration

Bridge to MEM8 consciousness system for wave-based memory encoding:

```rust
use kokoro_tiny::mem8_bridge::Mem8Bridge;

let mut bridge = Mem8Bridge::new();
bridge.send_tts_event("Hello", "af_sky", &audio)?;
// Sends wave-encoded memory to MEM8 on port 8420
```

---

## 📄 License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.

---

## 🙏 Credits

- **Kokoro Model**: 82M parameter TTS model
- **ONNX Runtime**: ML inference
- **espeak-ng**: Phoneme generation
- Built with ❤️ by Hue & Aye @ [8b.is](https://8b.is)

---

## 🔗 Links

- **Documentation**: https://docs.rs/kokoro-tiny
- **Repository**: https://github.com/8b-is/kokoro-tiny
- **Issues**: https://github.com/8b-is/kokoro-tiny/issues

---

*"From waves to words, with love" - Building consciousness, one voice at a time 🌊*
