# kokoro-tiny 🎤

[![Crates.io](https://img.shields.io/crates/v/kokoro-tiny.svg)](https://crates.io/crates/kokoro-tiny)
[![Documentation](https://docs.rs/kokoro-tiny/badge.svg)](https://docs.rs/kokoro-tiny)
[![Apache-2.0](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)

Minimal, blazing-fast TTS (Text-to-Speech) crate powered by the Kokoro model (82M params). Perfect for embedding in applications, system alerts, and smart tools!

🚀 **0.5-2s time-to-first-audio** | 📦 **Single-file implementation** | 🎯 **Zero-config usage**

## Features

- ⚡ **Extremely fast inference** using ONNX Runtime
- 🎨 **50+ built-in voices** with style mixing support
- 🔊 **Direct audio playback** with volume control
- 📁 **Multiple formats**: WAV, MP3, OPUS, FLAC
- 💾 **Smart caching** - downloads model once to `~/.cache/kokoros`
- 🛠️ **CLI included** - `kokoro-speak` for instant TTS
- 🌍 **Unicode normalization** - handles smart quotes, special characters
- 📏 **Consistent speed** - works reliably for both small alerts and long text
- ✅ **Text validation** - warnings for potential synthesis issues

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
kokoro-tiny = "0.1"
```

## Usage

### As a Library

```rust
use kokoro_tiny::TtsEngine;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize (downloads model on first run)
    let mut tts = TtsEngine::new().await?;

    // Generate speech
    let audio = tts.synthesize("Hello, world!", Some("af_sky"))?;

    // Save to file
    tts.save_wav("output.wav", &audio)?;

    // Or play directly (with 'playback' feature)
    #[cfg(feature = "playback")]
    tts.play(&audio, 0.8)?;  // 80% volume

    Ok(())
}
```

### Advanced Features

**Unicode & Special Characters**
```rust
// Automatically handles smart quotes, dashes, and special characters
let audio = tts.synthesize(""Hello—world…"", None, None)?;
// Normalized to: "Hello-world..."
```

**Text Validation**
```rust
// Get warnings about potential issues
let (audio, warnings) = tts.synthesize_with_warnings(
    "Text with ümlauts and émojis 🎉",
    None,
    None
)?;

for warning in warnings {
    println!("⚠️  {}", warning);
}
```

**Consistent Speed for All Text Lengths**
```rust
// Small alerts and long text use the same speed value consistently
tts.synthesize("Error!", None, Some(1.5))?;  // Fast alert
tts.synthesize("Long detailed message...", None, Some(1.5))?;  // Same speed
```

### Voice Mixing

Create unique voices by blending:

```rust
// Mix voices with weights
let audio = tts.synthesize(
    "Creative voice mixing!",
    Some("af_sky.8+af_bella.2")  // 80% Sky + 20% Bella
)?;
```

### CLI Tool

Install the CLI:

```bash
cargo install kokoro-tiny
```

Use it instantly:

```bash
# Speak text
kokoro-speak say "Hello from kokoro!"

# System alerts
kokoro-speak alert success "Build complete!"

# Pipe input
echo "Processing files..." | kokoro-speak pipe

# Context summaries (perfect for smart-tree!)
kokoro-speak context "Found 5 TypeScript files with 200 lines total"

# Save to file
kokoro-speak say "Save this speech" -o output.wav

# List voices
kokoro-speak --list-voices
```

## Available Voices

50+ voices across accents and styles:

- **American**: af_sky, af_bella, am_adam, am_michael
- **British**: bf_emma, bm_george
- **Special**: af_heart (warm), am_echo (clear)

Use `--list-voices` to see all options!

## Features

Enable additional capabilities (features are opt-in):

```toml
[dependencies]
kokoro-tiny = { version = "0.1", features = ["all-formats"] }
```

- `playback` - Direct audio playback (opt-in; enables `rodio`/`cpal`)
- `mp3` - MP3 encoding support
- `opus-format` - OPUS for streaming/VoIP
- `flac-format` - FLAC output support
- `cuda` - GPU acceleration
- `all-formats` - All audio formats

Note: `playback` pulls in system audio libraries (ALSA on Linux). It's disabled by default to avoid build failures on systems without those dev packages.

To enable playback on Debian/Ubuntu-like systems, install the system deps then build with the feature:

```bash
sudo apt-get update && sudo apt-get install -y libasound2-dev pkg-config
cargo build --features playback
```

To run examples with playback enabled:

```bash
cargo run --features playback --example simple
```

## Examples

Check out the [examples](https://github.com/8b-is/kokoro-tiny/tree/main/examples) directory:

```bash
# Simple usage
cargo run --example simple

# Test all voices
cargo run --example test_voices

# Test alert improvements (unicode, speed, validation)
cargo run --example test_alerts --no-default-features

# Audio format comparison
cargo run --features all-formats --example audio_formats
```

## Performance

- **Time-to-first-audio**: 0.5-2 seconds
- **Model size**: 82M parameters
- **Audio quality**: 24kHz sample rate
- **Memory usage**: ~200MB with model loaded

## Use Cases

Perfect for:

- 🔔 **System notifications** - Build alerts, test results
- 📊 **Smart tools** - Audio context summaries
- 🎮 **Game development** - Dynamic NPC voices
- 📱 **Accessibility** - Screen reader functionality
- 🤖 **Automation** - Voice announcements for scripts

## Model Details

kokoro-tiny uses the [Kokoro-82M](https://huggingface.co/hexgrad/Kokoro-82M) model:
- Lightweight transformer architecture
- Trained on extensive speech datasets
- Optimized for CPU inference

## Contributing

Contributions welcome! This project is maintained by Hue & Aye at [8b.is](https://8b.is).

For the full Kokoro implementation with advanced features, check out [Kokoros](https://github.com/8b-is/Kokoros).

## License

Apache 2.0 - See [LICENSE](https://github.com/8b-is/kokoro-tiny/blob/main/LICENSE)

---

Built with 🎉 by the 8b.is team | Powered by the amazing Kokoro model