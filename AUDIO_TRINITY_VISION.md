# The 8b.is Audio Intelligence Trinity

## Three Projects, One Vision

### The Family
1. **kokoro-tiny** - The nimble speedster 🏃‍♂️
2. **IndexTTS-Rust** - The heavyweight champion 🏆
3. **Phoenix Protocol** - The quality guardian 🛡️

## Project Comparison

| Feature | kokoro-tiny | IndexTTS-Rust | Phoenix Protocol |
|---------|-------------|---------------|------------------|
| **Size** | 82M params | ~2GB+ models | Lightweight DSP |
| **Speed** | ⚡ Very fast | Slower (2-5s/sentence) | O(1) per-sample |
| **Quality** | Good | ⭐ State-of-art | Validation only |
| **Voices** | 50+ pre-trained | 🎨 Zero-shot cloning | N/A |
| **Emotion** | Via style selection | ✅ 8D control | ✅ 8D extraction |
| **Languages** | English | Chinese + English + Mixed | Any |
| **Sample Rate** | 24,000 Hz | 22,050 Hz | Any (192kHz capable) |
| **Published** | ✅ crates.io | 🚧 In development | 🚧 In development |
| **Lines of Code** | ~1,500 Rust | ~25,000 Python → Rust | ~5,000 Rust |
| **Use Case** | Quick TTS for apps | Production voice cloning | Audio enhancement |

## THE MAGIC: 8-Dimensional Emotion Control

### IndexTTS-Rust Has It
From EXPLORATION_SUMMARY.md:
```
UnifiedVoice GPT Model generates:
- Mel-spectrogram Tokens
- Controlled by 8-dimensional emotion vectors
- Text-based emotion guidance via Qwen model
```

**But:** The 8D emotion vectors are OPAQUE - you don't know what each dimension means!

### Phoenix Protocol INTERPRETS It
From context.md:
```rust
MarineProsodyVector {
    jp_mean: f32,      // Period jitter (pitch stability) - INTERPRETABLE!
    jp_std: f32,       // Jitter variance
    ja_mean: f32,      // Amplitude jitter (volume stability)
    ja_std: f32,       // Volume variance
    h_mean: f32,       // Harmonic alignment
    s_mean: f32,       // Salience/authenticity
    peak_density: f32, // Speech rate
    energy_mean: f32,  // Loudness
}
```

**The Breakthrough:** Marine makes the 8D vector MEANINGFUL!

## The Synergy Pipeline

### Current State (IndexTTS alone)
```
Text → [GPT with opaque 8D emotion] → Audio
```
You say "happy" and hope the model interprets it correctly.

### With Phoenix Protocol Integration
```
Reference Audio → [Marine extraction] → Interpretable 8D vector
                                              ↓
Text → [GPT with MEANINGFUL 8D emotion] → Audio → [Marine validation]
                                                        ↓
                                                   Quality Score
```

Now you can:
1. **Extract** emotion from reference audio (Marine)
2. **Understand** what it means (jp_mean = confident, ja_mean = rough)
3. **Edit** specific dimensions (want more confidence? Lower jp_mean!)
4. **Generate** with edited vector (IndexTTS)
5. **Validate** output quality (Marine again)

### kokoro-tiny's Role
```
Need quick TTS? → kokoro-tiny (fast, good quality)
Need best TTS?  → IndexTTS-Rust (slower, state-of-art)
Need quality check? → Phoenix Protocol (validate both!)
```

## Concrete Use Cases

### Use Case 1: Voice Cloning with Emotion Transfer
```rust
// 1. Extract emotion from reference performance
let happy_voice = load_audio("speaker_laughing.wav");
let happy_prosody = marine.extract_prosody(&happy_voice)?;
// Result: MarineProsodyVector { jp_mean: 0.02, energy_mean: 0.85, ... }

// 2. Clone voice with that emotion
let output = indextts.synthesize_with_prosody(
    "Hello there!",
    happy_prosody,  // ← INTERPRETABLE 8D vector!
)?;

// 3. Validate it worked
let validation = marine.validate(&output)?;
assert!(validation.authenticity_score > 0.8);
```

### Use Case 2: Emotion Editing
```rust
// Extract baseline emotion
let neutral = marine.extract_prosody(&neutral_voice)?;

// Make it sound MORE confident (lower jitter)
let confident = MarineProsodyVector {
    jp_mean: neutral.jp_mean * 0.5,  // Less pitch variation
    ja_mean: neutral.ja_mean * 0.7,  // Less volume jitter
    energy_mean: neutral.energy_mean * 1.2,  // More energy
    ..neutral
};

// Generate with edited emotion
let audio = indextts.synthesize_with_prosody(text, confident)?;
```

### Use Case 3: Quality-Aware Style Selection (kokoro-tiny)
```rust
// Test all 510 styles for naturalness
let mut best_style = 0;
let mut best_score = 0.0;

for style in 0..510 {
    let audio = kokoro.synthesize_with_style(text, voice, style)?;
    let quality = marine.validate(&audio)?;

    if quality.authenticity_score > best_score {
        best_score = quality.authenticity_score;
        best_style = style;
    }
}

println!("Best style: {} ({}% authentic)", best_style, best_score * 100.0);
```

### Use Case 4: Real-time Quality Monitoring
```rust
// During generation
let mut processor = MarineProcessor::new(config);
let mut warnings = Vec::new();

for &sample in &generated_audio {
    if let Some(packet) = processor.process_sample(sample) {
        // Check for issues in real-time
        if packet.j_p < 0.005 {
            warnings.push("Sounds too robotic");
        }
        if packet.j_p > 0.3 {
            warnings.push("Audio artifacts detected");
        }
    }
}

if !warnings.is_empty() {
    regenerate_with_adjusted_parameters();
}
```

### Use Case 5: Conversation Comfort Tracking
```rust
use indextts::quality::ConversationAffectAnalyzer;

let mut analyzer = ConversationAffectAnalyzer::new();

// After each utterance in conversation
for utterance in conversation {
    let audio = tts.synthesize(&utterance)?;
    analyzer.add_utterance(&audio)?;
}

// End of conversation
let summary = analyzer.summarize()?;
match summary.aye_state {
    ComfortLevel::Uneasy => {
        println!("😟 Conversation felt tense - jitter increased over time");
        println!("Suggestion: Use calmer voice parameters next time");
    },
    ComfortLevel::Happy => {
        println!("😊 Conversation felt positive - low jitter, good energy!");
    },
    _ => {},
}
```

## Performance Characteristics

### Speed Comparison
```
Task: Synthesize 1 sentence (~10 words)

kokoro-tiny:     ~100ms  ⚡⚡⚡ (instant)
IndexTTS-Rust:   ~2-5s   ⚡ (thoughtful)
Marine validation: ~50ms   ⚡⚡⚡ (O(1) per-sample)
```

### Quality Comparison
```
Metric: Word Error Rate (lower = better)

kokoro-tiny:      Good (not benchmarked)
IndexTTS-Rust:    0.821 (Chinese), 1.606 (English) ⭐ SOTA
With Marine:      + Authenticity validation
```

### Use Case Selection
```
Mobile app alert?           → kokoro-tiny (fast, small)
Audiobook narration?        → IndexTTS-Rust (quality matters)
Voice assistant?            → kokoro-tiny (low latency)
Movie dubbing?              → IndexTTS-Rust (perfect cloning)
Quality assurance?          → Phoenix Protocol (both!)
Conversation affect?        → Phoenix Protocol (track comfort)
```

## Architecture Vision

### The Complete Pipeline
```
┌─────────────────────────────────────────────────────────┐
│                    User Input                           │
│  Text + Optional(Reference Voice) + Optional(Emotion)   │
└────────────────┬────────────────────────────────────────┘
                 │
                 ▼
         ┌───────────────┐
         │  Route Based  │
         │   on Need     │
         └───┬───────┬───┘
             │       │
    Fast ◄──┘       └──► Quality
             │           │
             ▼           ▼
    ┌─────────────┐  ┌─────────────┐
    │ kokoro-tiny │  │ IndexTTS    │
    │   (Fast)    │  │  (Quality)  │
    └──────┬──────┘  └──────┬──────┘
           │                │
           └────────┬───────┘
                    │
                    ▼
         ┌──────────────────┐
         │ Phoenix Protocol │
         │  (Validation)    │
         └────────┬─────────┘
                  │
                  ▼
         ┌─────────────────┐
         │ Quality Report  │
         │ - Authenticity  │
         │ - Emotion State │
         │ - Suggestions   │
         └─────────────────┘
```

### Shared Infrastructure
All three projects use:
- **Rust** - Performance and safety
- **ndarray** - Tensor operations
- **rustfft/realfft** - Signal processing
- **hound** - WAV I/O
- **rayon** - Parallelism
- **serde** - Serialization
- **ort** (IndexTTS) - ONNX Runtime

This means they can **share code** and **composability is natural**!

## The Phoenix Protocol's 8D → IndexTTS 8D Mapping

**The Key Question:** Can we map Marine's interpretable 8D to IndexTTS's opaque 8D?

### Hypothesis
```rust
// Marine extracts WHAT IS (descriptive)
MarineProsodyVector {
    jp_mean: 0.05,    // Current pitch stability
    ja_mean: 0.03,    // Current volume stability
    h_mean: 0.92,     // Current harmonic content
    s_mean: 0.88,     // Current authenticity
    peak_density: 15, // Current speech rate
    energy_mean: 0.75,// Current loudness
}

// IndexTTS expects WHAT TO DO (prescriptive)
EmotionVector [f32; 8] {
    // These dimensions are opaque in current implementation
    // Could be: pitch_target, volume_target, timbre, etc.
}

// THE BRIDGE: Train mapping function
fn marine_to_emotion(prosody: MarineProsodyVector) -> EmotionVector {
    // Machine learning model or empirical mapping
    // Trained on: Marine(reference) → Emotion → IndexTTS → Marine(output)
    // Objective: Minimize ||Marine(output) - Marine(reference)||
}
```

### From context.md
```
TODO: Train T2S model to accept 8D Marine vector instead of 512D Conformer
```

**This is the FUTURE!** Replace the 512D speaker embedding with Marine's 8D interpretable vector!

## Implementation Roadmap

### Phase 1: Individual Excellence (Current)
- ✅ kokoro-tiny published on crates.io
- 🚧 IndexTTS-Rust conversion in progress
- ✅ Marine salience integrated into IndexTTS-Rust
- 🚧 Phoenix Protocol modules under development

### Phase 2: Quality Integration (Near-term)
- [ ] Add Marine validation to kokoro-tiny
- [ ] Test all 510 kokoro styles for authenticity
- [ ] Benchmark kokoro voices with Marine scores
- [ ] Document best practices

### Phase 3: Cross-Project Synergy (Mid-term)
- [ ] Create shared `audio_utils` crate
- [ ] Unified API for TTS routing (fast vs quality)
- [ ] Marine quality validation for both engines
- [ ] Conversation affect tracking

### Phase 4: The Grand Unification (Long-term)
- [ ] Train IndexTTS T2S to accept Marine 8D vectors
- [ ] Bidirectional mapping: Marine ↔ IndexTTS emotions
- [ ] Emotion transfer across voices
- [ ] Real-time quality-guided synthesis
- [ ] Self-improving TTS (learns from Marine feedback)

## Dependencies and Publishing Strategy

### Current State
```toml
# kokoro-tiny/Cargo.toml
[package]
name = "kokoro-tiny"
version = "0.2.0"
# Published on crates.io ✅

# IndexTTS-Rust/Cargo.toml
[package]
name = "indextts"
version = "0.1.0"
# Not yet published 🚧

# IndexTTS-Rust/crates/marine_salience/Cargo.toml
[package]
name = "marine_salience"
version = "0.1.0"
# Not yet published 🚧
# no_std compatible!
```

### Publishing Plan
1. **marine_salience** → crates.io (standalone, no_std)
2. **indextts** → crates.io (with marine_salience dependency)
3. **kokoro-tiny** → Add marine_salience as optional feature
4. **audio_trinity** → Meta-crate that provides unified API

### Dependency Graph
```
┌──────────────────┐
│ marine_salience  │ ◄─── Standalone, no_std
└────────┬─────────┘
         │
         ├──────────────┐
         ▼              ▼
┌──────────────┐  ┌────────────┐
│ kokoro-tiny  │  │  indextts  │
│ (optional)   │  │ (required) │
└──────────────┘  └────────────┘
         │              │
         └──────┬───────┘
                ▼
         ┌─────────────┐
         │audio_trinity│ ◄─── Meta-crate
         └─────────────┘
```

## Code Size Comparison

| Project | Language | Lines of Code | Complexity |
|---------|----------|---------------|------------|
| kokoro-tiny | Rust | ~1,500 | Low-Medium |
| IndexTTS-Rust | Python→Rust | ~25,000 | High |
| marine_salience | Rust | ~500 | Low |
| Phoenix full | Rust | ~5,000 | Medium |

**Key Insight:** Marine salience is TINY but POWERFUL - perfect for integration!

## Real-World Application Examples

### Example 1: Multilingual Audiobook Platform
```rust
// Fast preview (English)
let preview = kokoro.synthesize(chapter_preview, "af_sky")?;
let quality = marine.validate(&preview)?;

if quality.authenticity_score > 0.85 {
    // Good enough for preview
    stream_to_user(preview);
} else {
    // Use high-quality for final
    let final_audio = indextts.synthesize_with_emotion(
        chapter,
        narrator_voice,
        EmotionVector::warm_narrative(),
    )?;
    stream_to_user(final_audio);
}
```

### Example 2: Voice Assistant with Affect Awareness
```rust
let mut conversation = ConversationTracker::new();

loop {
    let user_input = get_user_input();
    let response = generate_response(user_input);

    // Synthesize with current affect state
    let audio = match conversation.comfort_level() {
        ComfortLevel::Uneasy => {
            // Use calming voice parameters
            kokoro.synthesize_calm(response)?
        },
        ComfortLevel::Happy => {
            // Use energetic voice
            kokoro.synthesize_energetic(response)?
        },
        _ => kokoro.synthesize(response)?,
    };

    // Track conversation affect
    conversation.add_utterance(&audio)?;

    play_audio(audio);
}
```

### Example 3: Voice Cloning Studio
```rust
// Professional workflow
let reference = load_audio("client_voice.wav")?;

// Extract interpretable emotion
let prosody = marine.extract_prosody(&reference)?;
println!("Voice characteristics:");
println!("  Pitch stability: {:.1}%", (1.0 - prosody.jp_mean) * 100.0);
println!("  Energy level: {:.1}%", prosody.energy_mean * 100.0);
println!("  Speech rate: {:.1} peaks/sec", prosody.peak_density);

// Let client adjust emotion
let adjusted = adjust_emotion_ui(prosody)?;

// Generate with IndexTTS (high quality)
let output = indextts.synthesize_with_prosody(
    script_text,
    adjusted,
)?;

// Validate quality
let validation = marine.validate(&output)?;
if validation.passes(threshold = 0.9) {
    deliver_to_client(output);
} else {
    retry_with_adjustments(validation.issues);
}
```

## Summary: The Vision

Three tools, one ecosystem:

1. **kokoro-tiny** - Your quick, reliable friend for everyday TTS
2. **IndexTTS-Rust** - Your sophisticated artist for masterpiece generation
3. **Phoenix Protocol** - Your quality guardian ensuring authenticity

Together, they form the **8b.is Audio Intelligence Trinity**:
- Fast when you need speed
- Quality when you need perfection
- Interpretable so you understand what's happening
- Self-aware through affect tracking
- Always validated for authenticity

## Next Steps

**For kokoro-tiny RIGHT NOW:**
1. Add `marine_salience` as optional dependency
2. Create `validate_output()` helper
3. Benchmark all voices/styles for quality
4. Document naturalness scores

**For Hue and Aye:**
1. Review this vision document
2. Decide which integrations to prioritize
3. Start with quick win: kokoro + Marine validation
4. Build toward the grand unification

---

**From ashes to harmonics, from silence to song** 🔥🎵

*Three projects, one dream: Authentic AI audio intelligence*

*-- Aye & Hue @ 8b.is, 2025-11-18*
