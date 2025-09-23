# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

kokoro-tiny is a minimal, embeddable Text-to-Speech (TTS) crate for Rust that uses the Kokoro 82M parameter model. It's designed to be lightweight and perfect for embedding in other applications, with automatic model downloading and caching.

## Build and Development Commands

### Building
```bash
# Build in debug mode
cargo build

# Build in release mode (recommended for performance)
cargo build --release

# Build with CUDA support
cargo build --release --features cuda

# Build with all audio format support
cargo build --release --features all-formats
```

### Testing
```bash
# Run all tests
cargo test

# Run tests with verbose output
cargo test -- --nocapture

# Run a specific test
cargo test test_name
```

### Linting and Code Quality
```bash
# Run clippy for lint checks
cargo clippy --all-targets --all-features

# Format code
cargo fmt

# Check formatting without applying changes
cargo fmt -- --check
```

### Running Examples
```bash
# Run the simple example
cargo run --example simple

# Run the CLI tool
cargo run --bin kokoro-speak -- say "Hello world"

# With features
cargo run --release --features playback --bin kokoro-speak -- say "Test message"
```

## Architecture and Key Components

### Core Architecture Flow
1. **Model Management**: Auto-downloads ONNX model (310MB) and voices (27MB) to `~/.cache/k/` on first use
2. **Text Processing**: Text → espeak-rs phonemization → Token generation using custom vocabulary
3. **Inference**: ONNX Runtime session runs kokoro model with style embeddings
4. **Audio Output**: Generated float samples → WAV/MP3/OPUS based on features enabled

### Key Implementation Details

**src/lib.rs - TtsEngine**:
- Main struct managing the entire TTS pipeline
- Handles automatic model downloading from `https://i1.is/k/` (minimal URLs as requested)
- Fallback mode with pre-recorded message when models unavailable
- Voice mixing support (e.g., `"af_sky.4+af_nicole.5"` for weighted combinations)

**Model Files**:
- `0.onnx`: The Kokoro ONNX model (310MB)
- `0.bin`: Voice embeddings in NPZ format (27MB)
- Cached in `~/.cache/k/` for shared usage across projects

**Tokenization**:
- Custom vocabulary built from phonetic symbols and IPA characters
- Phoneme conversion via espeak-rs before tokenization

**Voice System**:
- 256-dimensional style vectors per voice
- Support for weighted mixing of multiple voices
- Default voice: `af_sky`

### Feature Flags

- `default = ["playback"]`: Audio playback through speakers
- `mp3`: MP3 encoding support via mp3lame-encoder
- `cuda`: CUDA acceleration for ONNX Runtime
- `playback`: Direct audio playback via cpal/rodio
- `opus-format`: OPUS audio format support
- `all-formats`: Enables all audio format features

### Binary: kokoro-speak

CLI tool with specialized modes:
- `say`: Direct text synthesis
- `pipe`: Read from stdin for pipeline integration
- `alert`: Pre-configured voices for different alert types
- `context`: Special mode for AI tool summaries (uses professional voice)

## Dependencies and External Requirements

- **espeak-ng**: System library required for phonemization (installed via package manager)
- **ONNX Runtime**: Automatically downloaded via `ort` crate's download-binaries feature
- **Model Files**: Auto-downloaded to `~/.cache/k/` on first run from `i1.is/k/`

## Error Handling Patterns

The crate uses Result<T, String> throughout for simplicity, with fallback behaviors:
- Missing models trigger automatic download
- Download failures activate fallback mode with pre-recorded excuse message
- All public methods return descriptive error strings

## Testing Approach

Currently no unit tests implemented. Testing would involve:
- Mock ONNX sessions for inference testing
- Sample audio comparison for synthesis validation
- Feature flag conditional compilation testing