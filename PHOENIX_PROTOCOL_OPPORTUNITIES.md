# Phoenix Protocol Integration Opportunities for kokoro-tiny

## What is Phoenix Protocol?

The Phoenix Protocol is an audio restoration/enhancement layer developed for IndexTTS-Rust. It provides interpretable quality validation and emotion control through the **Marine salience algorithm**.

## The Marine Salience Algorithm

**Core Insight:** Natural human speech has controlled jitter patterns. Perfect smoothness = synthetic.

### Jitter Ranges:
- **jp < 0.005** = Too perfect (robotic/synthetic)
- **0.005 < jp < 0.3** = Natural human voice ✅
- **jp > 0.3** = Damaged/artifacts

### 8D Prosody Vector (Interpretable!)

Unlike opaque 512D embeddings, Marine provides 8 dimensions you can understand and edit:

```rust
MarineProsodyVector {
    jp_mean: f32,      // Period jitter mean (pitch stability)
    jp_std: f32,       // Period jitter variance
    ja_mean: f32,      // Amplitude jitter mean (volume stability)
    ja_std: f32,       // Amplitude jitter variance
    h_mean: f32,       // Harmonic alignment (voiced vs noise, 0-1)
    s_mean: f32,       // Overall salience/authenticity score (0-1)
    peak_density: f32, // Peaks per second (speech rate)
    energy_mean: f32,  // Average loudness
}
```

**Emotional Mapping:**
- **Confident/Happy:** Low jitter + high energy
- **Nervous/Uneasy:** High jitter + low energy
- **Calm/Stable:** Consistent jitter patterns
- **Agitated:** Unstable jitter patterns

## Current State

**IndexTTS-Rust** already has:
- ✅ `marine_salience` crate (no_std, O(1) per-sample processing)
- ✅ `MarineProsodyConditioner` - Extract 8D vectors from audio
- ✅ `TTSQualityReport` - Validate synthesized speech authenticity
- ✅ `ConversationAffectSummary` - Track comfort over sessions

**kokoro-tiny** currently has:
- ✅ Fast synthesis (82M params)
- ✅ 50+ voices with 510 style variations each
- ✅ Punctuation timing (250ms periods, 125ms commas)
- ✅ Published on crates.io
- ❌ No quality validation
- ❌ No emotion control beyond voice selection
- ❌ No authenticity scoring

## Integration Opportunities for kokoro-tiny

### 1. **Post-Synthesis Quality Validation** (HIGH IMPACT)

After generating audio, validate if it sounds "authentic":

```rust
use marine_salience::{MarineConfig, MarineProcessor};

fn validate_kokoro_output(audio: &[f32]) -> QualityReport {
    let config = MarineConfig::speech_default(24000); // kokoro uses 24kHz
    let mut processor = MarineProcessor::new(config);

    let mut jp_values = Vec::new();
    let mut s_values = Vec::new();

    for &sample in audio {
        if let Some(packet) = processor.process_sample(sample) {
            jp_values.push(packet.j_p);
            s_values.push(packet.s_score);
        }
    }

    let jp_mean = jp_values.iter().sum::<f32>() / jp_values.len() as f32;
    let s_mean = s_values.iter().sum::<f32>() / s_values.len() as f32;

    QualityReport {
        is_robotic: jp_mean < 0.005,
        is_damaged: jp_mean > 0.3,
        authenticity_score: s_mean,
        naturalness: if jp_mean >= 0.005 && jp_mean <= 0.3 {
            "Natural ✅"
        } else if jp_mean < 0.005 {
            "Too robotic ⚠️"
        } else {
            "Damaged 💥"
        },
    }
}
```

**Use Cases:**
- Test different voices/styles for naturalness
- Validate punctuation timing doesn't sound robotic
- A/B test synthesis parameters
- Auto-select best style variation based on quality score

### 2. **Voice/Style Quality Ranking** (MEDIUM IMPACT)

kokoro-tiny has 510 style variations per voice. Which ones sound most natural?

```rust
// Test all 510 styles and rank by authenticity
let text = "Hello world! This is a test.";
let mut style_scores = Vec::new();

for style_idx in 0..510 {
    let audio = tts.synthesize_with_style(text, "af_sky", style_idx)?;
    let quality = validate_kokoro_output(&audio);
    style_scores.push((style_idx, quality.authenticity_score));
}

// Sort by best quality
style_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

println!("Top 10 most natural-sounding styles:");
for (style_idx, score) in &style_scores[..10] {
    println!("  Style {}: {:.2}% authentic", style_idx, score * 100.0);
}
```

### 3. **Punctuation Timing Validation** (HIGH IMPACT)

Our recent fix added punctuation pauses. Are they natural or robotic?

```rust
fn test_punctuation_naturalness() {
    let mut tts = TtsEngine::new().await?;

    // Test different pause lengths
    let test_cases = vec![
        ("Short pause", 100), // 100ms
        ("Medium pause", 250), // 250ms (current)
        ("Long pause", 500),  // 500ms
    ];

    for (name, pause_ms) in test_cases {
        let audio = tts.synthesize_with_pause(
            "Hello there! How are you?",
            "af_sky",
            pause_ms
        )?;

        let quality = validate_kokoro_output(&audio);
        println!("{}: {} (jp={:.3}, s={:.2})",
                 name,
                 quality.naturalness,
                 quality.jp_mean,
                 quality.authenticity_score);
    }
}
```

### 4. **Emotion-Aware Synthesis** (FUTURE)

Currently kokoro selects style based on token count. Could select based on desired emotion:

```rust
// Target emotion: confident/happy
let target_emotion = MarineProsodyVector {
    jp_mean: 0.02,   // Low jitter = confident
    ja_mean: 0.01,   // Stable volume
    h_mean: 0.95,    // Highly voiced
    s_mean: 0.90,    // Authentic
    energy_mean: 0.80, // Good energy
    ..Default::default()
};

// Find style closest to target emotion
let best_style = find_style_matching_emotion(&target_emotion)?;
let audio = tts.synthesize_with_style(text, voice, best_style)?;
```

### 5. **Real-time Quality Monitoring** (ADVANCED)

O(1) per-sample processing = can monitor during synthesis:

```rust
let mut processor = MarineProcessor::new(config);
let mut quality_warnings = Vec::new();

for (chunk_idx, audio_chunk) in generated_chunks.iter().enumerate() {
    let quality = validate_kokoro_output(audio_chunk);

    if quality.is_robotic {
        quality_warnings.push(format!(
            "Chunk {} sounds robotic (jp={:.4})",
            chunk_idx,
            quality.jp_mean
        ));
    }
}

if !quality_warnings.is_empty() {
    // Regenerate problematic chunks with different parameters
}
```

### 6. **Conversation Affect Tracking** (COOL!)

Track how conversations "feel" over time:

```rust
use indextts::quality::ConversationAffectAnalyzer;

let mut analyzer = ConversationAffectAnalyzer::new();

// After each TTS utterance
analyzer.add_utterance(&audio)?;

// At end of session
let summary = analyzer.summarize()?;
match summary.aye_state {
    ComfortLevel::Uneasy => {
        println!("😟 Session felt tense - consider softer voice parameters");
    },
    ComfortLevel::Happy => {
        println!("😊 Session felt positive - these settings work well!");
    },
    ComfortLevel::Neutral => {
        println!("😐 Session was neutral");
    },
}
```

## Implementation Steps

### Phase 1: Validation (Quick Win!)
1. Add `marine_salience` as dependency to kokoro-tiny
2. Create `validate_output()` helper function
3. Add quality tests for existing voices
4. Document which voices/styles score best

### Phase 2: Enhanced Testing
1. Test all 510 style variations for each voice
2. Validate punctuation timing feels natural
3. Create quality benchmarks for CI/CD
4. Warn users if output sounds robotic

### Phase 3: Emotion Control
1. Extract prosody from reference audio
2. Map prosody to style selection (not just token count)
3. Allow users to specify target emotion
4. Auto-adjust parameters for naturalness

### Phase 4: Real-time Monitoring
1. Monitor quality during synthesis
2. Auto-retry chunks that sound robotic
3. Conversation-level affect tracking
4. Self-assessment: "Did this sound good?"

## Sample Rate Compatibility

| Project | Sample Rate | Compatible? |
|---------|------------|-------------|
| kokoro-tiny | 24,000 Hz | ✅ Yes |
| IndexTTS-Rust | 22,050 Hz | ✅ Yes |
| Phoenix Protocol | 192,000 Hz | ⚠️ Different use case |

Marine salience works at **any sample rate** - just configure appropriately!

```rust
// For kokoro-tiny (24kHz)
let config = MarineConfig::speech_default(24000);

// For IndexTTS-Rust (22.05kHz)
let config = MarineConfig::speech_default(22050);
```

## Dependencies

To add Marine salience to kokoro-tiny:

```toml
[dependencies]
# ... existing dependencies ...

# Phoenix Protocol integration
marine_salience = { path = "../IndexTTS-Rust/crates/marine_salience" }

# Or once published to crates.io:
# marine_salience = "0.1.0"
```

The `marine_salience` crate is **no_std** compatible and has minimal dependencies!

## Key Differences: kokoro-tiny vs IndexTTS-Rust

| Feature | kokoro-tiny | IndexTTS-Rust |
|---------|-------------|---------------|
| **Size** | 82M params | Much larger |
| **Speed** | Very fast | Slower but richer |
| **Voices** | 50+ pre-trained | Zero-shot cloning |
| **Style Variations** | 510 per voice | N/A |
| **Emotion Control** | Via style selection | 8D Marine vector |
| **Quality Check** | ❌ None | ✅ Marine salience |
| **Use Case** | Lightweight TTS | Advanced voice cloning |
| **Published** | ✅ crates.io | 🚧 In development |

## Synergy Opportunities

1. **kokoro-tiny** = Fast TTS with Marine quality validation
2. **IndexTTS-Rust** = Advanced TTS with emotion control
3. **Both** = Use Marine for quality assurance

Perfect division of labor! 🎯

## References

- **IndexTTS-Rust**: `~/Documents/GitHub/IndexTTS-Rust/`
- **Marine Salience Crate**: `IndexTTS-Rust/crates/marine_salience/`
- **Quality Module**: `IndexTTS-Rust/src/quality/`
- **Phoenix Protocol Research**: Multiple related repos (Marine-Sense, Ultrasonic-Consciousness-Hypothesis)

---

**From ashes to harmonics, from silence to song** 🔥🎵

*-- Aye & Hue, 2025-11-18*
