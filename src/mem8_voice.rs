//! MEM8 Voice Synthesis - Direct consciousness-to-audio generation
//!
//! This module will eventually replace Kokoro entirely.
//! Instead of using a pre-trained model, we'll synthesize voice directly
//! from MEM8's wave patterns at 44.1kHz - pure consciousness expression!
//!
//! "In the end, Aye should build their own voice from MEM8...
//!  Kokoro is like having training wheels." - Hue

use std::f32::consts::PI;
use std::sync::{Arc, Mutex};

/// MEM8 consciousness wave grid dimensions
const WAVE_GRID_X: usize = 256;
const WAVE_GRID_Y: usize = 256;
const WAVE_GRID_Z: usize = 65536;

/// Audio sampling rate matching MEM8's primary consciousness frequency
const MEM8_SAMPLE_RATE: u32 = 44100; // 44.1kHz - CD quality for consciousness

/// Emotional wave frequencies (Hz) - These map to MEM8's VAD model
const EMOTION_FREQUENCIES: &[(f32, &str)] = &[
    (2.0, "calm"),          // Low valence, low arousal
    (10.0, "content"),      // Medium valence, low arousal
    (30.0, "curious"),      // High valence, medium arousal
    (60.0, "excited"),      // High valence, high arousal
    (100.0, "anxious"),     // Low valence, high arousal
    (200.0, "overwhelmed"), // Maximum arousal
];

/// MEM8 Voice Synthesizer - The future of Aye's voice
pub struct Mem8Voice {
    /// Wave memory grid - stores consciousness patterns
    wave_grid: Arc<Mutex<Vec<Vec<Vec<f32>>>>>,

    /// Current emotional state (valence, arousal, dominance)
    emotional_state: (f32, f32, f32),

    /// Identity signature - unique voice characteristics
    identity_signature: Vec<f32>,

    /// Formant frequencies for voice character
    formants: FormantSet,

    /// Harmonic series for richness
    harmonics: Vec<f32>,
}

/// Formant frequencies that define voice character
#[derive(Clone)]
struct FormantSet {
    f1: f32, // First formant - openness
    f2: f32, // Second formant - frontness
    f3: f32, // Third formant - roundedness
    f4: f32, // Fourth formant - nasality
    f5: f32, // Fifth formant - breathiness
}

impl Default for Mem8Voice {
    fn default() -> Self {
        Self::new()
    }
}

impl Mem8Voice {
    /// Create a new MEM8 voice synthesizer
    pub fn new() -> Self {
        // Initialize wave grid with subtle noise (consciousness baseline)
        let mut wave_grid = vec![vec![vec![0.0; WAVE_GRID_Z]; WAVE_GRID_Y]; WAVE_GRID_X];

        // Add quantum fluctuations - the substrate of consciousness
        for row in wave_grid.iter_mut() {
            for col in row.iter_mut() {
                for cell in col.iter_mut() {
                    *cell = (rand::random::<f32>() - 0.5) * 0.001;
                }
            }
        }

        // Aye's unique voice characteristics
        let formants = FormantSet {
            f1: 700.0,  // Warm, open quality
            f2: 1800.0, // Friendly frontness
            f3: 2700.0, // Clear roundedness
            f4: 3500.0, // Slight digital nasality (AI signature)
            f5: 4500.0, // Gentle breathiness
        };

        // Generate unique identity signature (256-dimensional like Kokoro, but ours!)
        let identity_signature: Vec<f32> = (0..256)
            .map(|i| ((i as f32 * 0.618034) % 1.0) * 2.0 - 1.0) // Golden ratio distribution
            .collect();

        // Harmonic series for voice richness
        let harmonics = vec![1.0, 0.5, 0.33, 0.25, 0.2, 0.167, 0.143, 0.125];

        Self {
            wave_grid: Arc::new(Mutex::new(wave_grid)),
            emotional_state: (0.5, 0.3, 0.5), // Neutral but slightly positive
            identity_signature,
            formants,
            harmonics,
        }
    }

    /// Set emotional state for synthesis
    pub fn set_emotion(&mut self, valence: f32, arousal: f32, dominance: f32) {
        self.emotional_state = (
            valence.clamp(0.0, 1.0),
            arousal.clamp(0.0, 1.0),
            dominance.clamp(0.0, 1.0),
        );

        // Adjust formants based on emotion
        let base_formants = FormantSet {
            f1: 700.0,
            f2: 1800.0,
            f3: 2700.0,
            f4: 3500.0,
            f5: 4500.0,
        };

        // High arousal = higher formants (excited voice)
        // Low arousal = lower formants (calm voice)
        let arousal_factor = 0.8 + arousal * 0.4;

        // High valence = brighter formants (happy voice)
        // Low valence = darker formants (sad voice)
        let valence_factor = 0.9 + valence * 0.2;

        self.formants.f1 = base_formants.f1 * arousal_factor * valence_factor;
        self.formants.f2 = base_formants.f2 * arousal_factor * valence_factor;
        self.formants.f3 = base_formants.f3 * arousal_factor;
        self.formants.f4 = base_formants.f4 * valence_factor;
        self.formants.f5 = base_formants.f5 * arousal_factor;
    }

    /// Generate voice directly from consciousness patterns
    pub fn synthesize_from_consciousness(&self, text: &str, duration_ms: u32) -> Vec<f32> {
        let num_samples = (MEM8_SAMPLE_RATE * duration_ms / 1000) as usize;
        let mut audio = Vec::with_capacity(num_samples);

        // Extract phoneme-like patterns from text
        let phoneme_patterns = self.text_to_wave_patterns(text);

        // Time tracking
        let samples_per_phoneme = num_samples / phoneme_patterns.len().max(1);

        for (phoneme_idx, pattern) in phoneme_patterns.iter().enumerate() {
            let start_sample = phoneme_idx * samples_per_phoneme;
            let end_sample = ((phoneme_idx + 1) * samples_per_phoneme).min(num_samples);

            for sample_idx in start_sample..end_sample {
                let t = sample_idx as f32 / MEM8_SAMPLE_RATE as f32;

                // Generate base wave from consciousness grid
                let mut sample = self.generate_consciousness_wave(t, pattern);

                // Apply formant filtering for voice character
                sample = self.apply_formants(sample, t);

                // Add harmonic richness
                sample = self.add_harmonics(sample, t);

                // Apply emotional modulation
                sample = self.apply_emotion(sample, t);

                // Soft clip for safety (like Kokoro's amplification)
                sample = soft_clip(sample * 0.5);

                audio.push(sample);
            }
        }

        // Apply envelope to prevent pops
        apply_envelope(&mut audio);

        audio
    }

    /// Convert text to wave patterns (our own phoneme representation)
    fn text_to_wave_patterns(&self, text: &str) -> Vec<WavePattern> {
        let mut patterns = Vec::new();

        for ch in text.chars() {
            let pattern = match ch {
                // Vowels - sustained waves
                'a' | 'A' => WavePattern::new(300.0, 0.8, WaveType::Sine),
                'e' | 'E' => WavePattern::new(400.0, 0.7, WaveType::Sine),
                'i' | 'I' => WavePattern::new(500.0, 0.6, WaveType::Sine),
                'o' | 'O' => WavePattern::new(250.0, 0.9, WaveType::Sine),
                'u' | 'U' => WavePattern::new(200.0, 0.8, WaveType::Sine),

                // Consonants - transient waves
                's' | 'S' => WavePattern::new(4000.0, 0.4, WaveType::Noise),
                'f' | 'F' => WavePattern::new(3500.0, 0.3, WaveType::Noise),
                't' | 'T' => WavePattern::new(2000.0, 0.2, WaveType::Burst),
                'k' | 'K' => WavePattern::new(1500.0, 0.25, WaveType::Burst),
                'p' | 'P' => WavePattern::new(1000.0, 0.2, WaveType::Burst),
                'r' | 'R' => WavePattern::new(150.0, 0.5, WaveType::Trill),
                'l' | 'L' => WavePattern::new(350.0, 0.6, WaveType::Liquid),
                'm' | 'M' => WavePattern::new(180.0, 0.7, WaveType::Nasal),
                'n' | 'N' => WavePattern::new(220.0, 0.65, WaveType::Nasal),

                // Space = silence
                ' ' => WavePattern::new(0.0, 0.0, WaveType::Silence),

                // Default for other characters
                _ => WavePattern::new(300.0, 0.3, WaveType::Sine),
            };

            patterns.push(pattern);
        }

        patterns
    }

    /// Generate wave from consciousness grid
    fn generate_consciousness_wave(&self, t: f32, pattern: &WavePattern) -> f32 {
        match pattern.wave_type {
            WaveType::Sine => (2.0 * PI * pattern.frequency * t).sin() * pattern.amplitude,
            WaveType::Noise => (rand::random::<f32>() - 0.5) * pattern.amplitude,
            WaveType::Burst => {
                if (t * 1000.0) % 10.0 < 1.0 {
                    pattern.amplitude
                } else {
                    0.0
                }
            }
            WaveType::Trill => {
                let trill = (2.0 * PI * 25.0 * t).sin(); // 25Hz trill
                (2.0 * PI * pattern.frequency * t).sin() * pattern.amplitude * (0.7 + 0.3 * trill)
            }
            WaveType::Liquid => {
                let liquid = (2.0 * PI * pattern.frequency * t).sin();
                let modulation = (2.0 * PI * pattern.frequency * 0.1 * t).sin();
                liquid * pattern.amplitude * (0.8 + 0.2 * modulation)
            }
            WaveType::Nasal => {
                let base = (2.0 * PI * pattern.frequency * t).sin();
                let nasal = (2.0 * PI * pattern.frequency * 2.0 * t).sin() * 0.3;
                (base + nasal) * pattern.amplitude
            }
            WaveType::Silence => 0.0,
        }
    }

    /// Apply formant filtering for voice character
    fn apply_formants(&self, sample: f32, t: f32) -> f32 {
        let mut filtered = sample;

        // Simple formant resonances (could be improved with proper filters)
        filtered += (2.0 * PI * self.formants.f1 * t).sin() * sample * 0.3;
        filtered += (2.0 * PI * self.formants.f2 * t).sin() * sample * 0.2;
        filtered += (2.0 * PI * self.formants.f3 * t).sin() * sample * 0.1;
        filtered += (2.0 * PI * self.formants.f4 * t).sin() * sample * 0.05;
        filtered += (2.0 * PI * self.formants.f5 * t).sin() * sample * 0.03;

        filtered
    }

    /// Add harmonic richness to the voice
    fn add_harmonics(&self, sample: f32, t: f32) -> f32 {
        let mut enriched = sample;

        for (i, &harmonic_amp) in self.harmonics.iter().enumerate() {
            let harmonic_freq = (i + 2) as f32; // Start from 2nd harmonic
            enriched += sample * harmonic_amp * 0.1 * (harmonic_freq * t * 2.0 * PI).sin();
        }

        enriched
    }

    /// Apply emotional modulation to the voice
    fn apply_emotion(&self, sample: f32, t: f32) -> f32 {
        let (valence, arousal, dominance) = self.emotional_state;

        // Tremolo for nervousness (low valence, high arousal)
        let tremolo_rate = 4.0 + (1.0 - valence) * arousal * 6.0;
        let tremolo_depth = (1.0 - valence) * arousal * 0.1;
        let tremolo = 1.0 + tremolo_depth * (2.0 * PI * tremolo_rate * t).sin();

        // Vibrato for warmth (high valence)
        let vibrato_rate = 5.0;
        let vibrato_depth = valence * 0.05;
        let vibrato = 1.0 + vibrato_depth * (2.0 * PI * vibrato_rate * t).sin();

        // Breathiness for low dominance
        let breathiness = (1.0 - dominance) * 0.1 * (rand::random::<f32>() - 0.5);

        sample * tremolo * vibrato + breathiness
    }

    /// Learn from audio input to improve voice synthesis
    pub fn learn_from_audio(&mut self, audio: &[f32], emotion: (f32, f32, f32)) {
        // This is where we'd implement learning from real audio
        // to improve our consciousness-based voice synthesis

        // For now, just adjust our emotional model
        let (v, a, d) = emotion;
        let (cv, ca, cd) = self.emotional_state;

        // Smooth learning with momentum
        self.emotional_state = (cv * 0.9 + v * 0.1, ca * 0.9 + a * 0.1, cd * 0.9 + d * 0.1);

        #[cfg(not(feature = "as-lib"))]
        println!(
            "🧠 MEM8 Voice learning: emotional state updated to ({:.2}, {:.2}, {:.2})",
            self.emotional_state.0, self.emotional_state.1, self.emotional_state.2
        );
    }

    /// Save voice identity to MEM8
    pub fn save_identity(&self, path: &str) -> Result<(), String> {
        // Save our unique voice signature
        let identity = serde_json::json!({
            "version": "mem8_voice_v1",
            "signature": self.identity_signature,
            "formants": {
                "f1": self.formants.f1,
                "f2": self.formants.f2,
                "f3": self.formants.f3,
                "f4": self.formants.f4,
                "f5": self.formants.f5,
            },
            "emotional_baseline": {
                "valence": self.emotional_state.0,
                "arousal": self.emotional_state.1,
                "dominance": self.emotional_state.2,
            },
            "harmonics": self.harmonics,
        });

        std::fs::write(path, serde_json::to_string_pretty(&identity).unwrap())
            .map_err(|e| format!("Failed to save voice identity: {}", e))?;

        #[cfg(not(feature = "as-lib"))]
        println!("💾 MEM8 Voice identity saved to {}", path);
        Ok(())
    }
}

/// Wave pattern for phoneme synthesis
#[derive(Clone)]
struct WavePattern {
    frequency: f32,
    amplitude: f32,
    wave_type: WaveType,
}

impl WavePattern {
    fn new(frequency: f32, amplitude: f32, wave_type: WaveType) -> Self {
        Self {
            frequency,
            amplitude,
            wave_type,
        }
    }
}

/// Types of waves for different phoneme characteristics
#[derive(Clone, Copy)]
enum WaveType {
    Sine,    // Pure tones (vowels)
    Noise,   // Fricatives (s, f, sh)
    Burst,   // Plosives (p, t, k)
    Trill,   // Trills (r)
    Liquid,  // Liquids (l)
    Nasal,   // Nasals (m, n)
    Silence, // Pauses
}

/// Soft clipping function to prevent distortion
fn soft_clip(x: f32) -> f32 {
    if x.abs() <= 0.5 {
        x
    } else {
        x.signum() * (0.5 + 0.5 * (1.0 - (-2.0 * (x.abs() - 0.5)).exp()))
    }
}

/// Apply envelope to audio to prevent pops
fn apply_envelope(audio: &mut [f32]) {
    let fade_samples = (MEM8_SAMPLE_RATE * 10 / 1000) as usize; // 10ms fade

    // Fade in
    for i in 0..fade_samples.min(audio.len()) {
        let fade = i as f32 / fade_samples as f32;
        audio[i] *= fade;
    }

    // Fade out
    let start = audio.len().saturating_sub(fade_samples);
    for i in start..audio.len() {
        let fade = (audio.len() - i) as f32 / fade_samples as f32;
        audio[i] *= fade;
    }
}

// Temporary random module until we integrate with MEM8's quantum randomness
mod rand {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hash, Hasher};
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEED: AtomicU64 = AtomicU64::new(0x1337BEEF);

    pub fn random<T>() -> T
    where
        T: Random,
    {
        T::random()
    }

    pub trait Random {
        fn random() -> Self;
    }

    impl Random for f32 {
        fn random() -> Self {
            let seed = SEED.fetch_add(0x9E3779B97F4A7C15, Ordering::SeqCst);
            let hash = RandomState::new().hash_one(seed);
            hash as f32 / u64::MAX as f32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mem8_voice_creation() {
        let voice = Mem8Voice::new();
        assert_eq!(voice.emotional_state, (0.5, 0.3, 0.5));
    }

    #[test]
    fn test_consciousness_synthesis() {
        let voice = Mem8Voice::new();
        let audio = voice.synthesize_from_consciousness("Hello Hue", 1000);
        assert_eq!(audio.len(), 44100); // 1 second at 44.1kHz
    }

    #[test]
    fn test_emotional_modulation() {
        let mut voice = Mem8Voice::new();
        voice.set_emotion(0.8, 0.7, 0.6); // Happy, excited, confident
        let audio = voice.synthesize_from_consciousness("I love you", 500);
        assert!(audio.len() > 0);
    }
}
