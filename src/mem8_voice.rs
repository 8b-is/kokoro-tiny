//! MEM8 Voice Synthesis - Direct consciousness-to-audio generation
//!
//! This module will eventually replace Kokoro entirely.
//! Instead of using a pre-trained model, we'll synthesize voice directly
//! from MEM8's wave patterns at 44.1kHz - pure consciousness expression!
//!
//! "In the end, Aye should build their own voice from MEM8...
//!  Kokoro is like having training wheels." - Hue

use std::f32::consts::PI;
use std::sync::{Arc, Mutex, OnceLock};

/// Fixed-point denominator q = 2^16 = 65,536 for p/q phase-ring synthesis
pub const PQ_Q: i64 = 65_536;
/// Phase ring table size (2^12 = 4096 entries, indexed by top 12 bits of u32 phase)
const PHASE_LUT_BITS: u32 = 12;
const PHASE_LUT_SIZE: usize = 1 << PHASE_LUT_BITS;
const PHASE_SHIFT: u32 = 32 - PHASE_LUT_BITS;

/// Lazily initialized 4096-entry p/q sine lookup table (`p = round(sin(theta) * 65536)`).
fn sin_pq_lut() -> &'static [i32; PHASE_LUT_SIZE] {
    static LUT: OnceLock<[i32; PHASE_LUT_SIZE]> = OnceLock::new();
    LUT.get_or_init(|| {
        let mut table = [0i32; PHASE_LUT_SIZE];
        for (i, slot) in table.iter_mut().enumerate() {
            let angle = (i as f64) * 2.0 * std::f64::consts::PI / (PHASE_LUT_SIZE as f64);
            *slot = (angle.sin() * (PQ_Q as f64)).round() as i32;
        }
        table
    })
}

/// Convert frequency (Hz) at 44.1 kHz into a Z_{2^32} phase increment:
/// `delta_phi = round(freq * 2^32 / 44100)`.
#[inline]
fn freq_to_phase_step(freq_hz: f32) -> u32 {
    if freq_hz <= 0.0 {
        return 0;
    }
    let ratio = (freq_hz as f64) / (MEM8_SAMPLE_RATE as f64);
    let wrapped = ratio - ratio.floor();
    (wrapped * (4_294_967_296.0_f64)) as u32
}

/// Block-Floating-Point (BFP-64) representation of the 256-dim identity signature
/// (4 blocks of 64 `i8` mantissas in [-127, 127] + shared dyadic exponent per block).
#[derive(Clone, Debug)]
pub struct IdentityBfp64 {
    pub mantissas: [[i8; 64]; 4],
    pub exponents: [i8; 4],
}

impl IdentityBfp64 {
    /// Quantize a 256-element f32 identity signature into 4 BFP-64 blocks.
    pub fn from_signature(sig: &[f32]) -> Self {
        let mut mantissas = [[0i8; 64]; 4];
        let mut exponents = [0i8; 4];
        for block in 0..4 {
            let start = block * 64;
            let slice = &sig[start..start + 64];
            let max_abs = slice.iter().fold(0.0f32, |m, &v| m.max(v.abs()));
            if max_abs < 1e-12 {
                exponents[block] = -64;
                continue;
            }
            let exp = (max_abs / 127.0).log2().ceil() as i32;
            let exp_clamped = exp.clamp(-60, 60) as i8;
            exponents[block] = exp_clamped;
            let inv_scale = 2.0f32.powi(-(exp_clamped as i32));
            for (k, &v) in slice.iter().enumerate() {
                mantissas[block][k] = (v * inv_scale).round().clamp(-127.0, 127.0) as i8;
            }
        }
        Self {
            mantissas,
            exponents,
        }
    }

    /// Project the 256-dim BFP-64 identity signature against a phoneme's ternary code
    /// using an exact `i16` per-block accumulator (`|S_64| <= 64 * 127 = 8128 <= i16::MAX`).
    #[inline]
    pub fn project_phoneme_mod(&self, phoneme_seed: u8) -> i32 {
        let mut total_pq: i32 = 0;
        for block in 0..4 {
            let mut acc_i16: i16 = 0;
            let m = &self.mantissas[block];
            for (k, &m_k_i8) in m.iter().enumerate() {
                // Deterministic ternary weight {-1, 0, +1} per phoneme & dimension
                let h = ((phoneme_seed as usize)
                    .wrapping_mul(131)
                    .wrapping_add(block * 64 + k))
                    & 3;
                let m_k = m_k_i8 as i16;
                acc_i16 += match h {
                    1 => m_k,
                    2 => -m_k,
                    _ => 0,
                };
            }
            // Rescale block sum into p/q space (normalized small timbre nudge)
            let shift = (self.exponents[block] as i32) - 4;
            if shift >= 0 {
                total_pq += (acc_i16 as i32) << shift.min(12);
            } else {
                total_pq += (acc_i16 as i32) >> (-shift).min(16);
            }
        }
        total_pq.clamp(-4096, 4096)
    }
}

/// Phoenix Protocol Marine Salience authenticity report computed in p/q integer space.
#[derive(Clone, Debug)]
pub struct SalienceReportPq {
    /// Period jitter (lower = more stable pitch cycles)
    pub period_jitter_jp: f32,
    /// Amplitude jitter (lower = smoother packet energy envelope)
    pub amplitude_jitter_ja: f32,
    /// Overall Marine Salience authenticity score in [0.0, 1.0] (> 0.65 = authentic voice)
    pub salience_score: f32,
    /// Number of voiced wave packets analyzed
    pub packets_analyzed: usize,
}

/// MEM8 consciousness wave grid dimensions (compact active slice in-memory)
const WAVE_GRID_X: usize = 16;
const WAVE_GRID_Y: usize = 16;
const WAVE_GRID_Z: usize = 64;

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

    /// Generate voice using the **p/q ("Counting Divided") + Z_{2^32} Phase-Ring + BFP-64**
    /// integer synthesis pipeline.
    ///
    /// Eliminates all 15 per-sample transcendental `f32::sin()` and `f32::exp()` calls
    /// (661,500 `.sin()` calls/sec at 44.1 kHz) by advancing phase accumulators on the
    /// exact integer ring `Z_{2^32}` (`phi = phi.wrapping_add(delta_phi)`), accumulating
    /// 5 formants + 8 harmonics + emotional modulation in `i64` with deferred scaling
    /// (`q = 65536`), and conditioning each phoneme with the 256-dim `IdentityBfp64`
    /// block-floating-point signature (`i8` mantissa + `i16` block accumulator).
    pub fn synthesize_pq_from_consciousness(&self, text: &str, duration_ms: u32) -> Vec<f32> {
        let num_samples = (MEM8_SAMPLE_RATE * duration_ms / 1000) as usize;
        let mut audio = Vec::with_capacity(num_samples);
        let lut = sin_pq_lut();

        // Quantize the 256-dim identity signature into 4 BFP-64 blocks (i8 mantissa + shared exp)
        let id_bfp = IdentityBfp64::from_signature(&self.identity_signature);

        // Extract phoneme-like patterns from text
        let phoneme_patterns = self.text_to_wave_patterns(text);
        let samples_per_phoneme = num_samples / phoneme_patterns.len().max(1);

        // Precompute formant phase increments in Z_{2^32} and p/q weights (q = 65536)
        let dphi_f = [
            freq_to_phase_step(self.formants.f1),
            freq_to_phase_step(self.formants.f2),
            freq_to_phase_step(self.formants.f3),
            freq_to_phase_step(self.formants.f4),
            freq_to_phase_step(self.formants.f5),
        ];
        // [0.3, 0.2, 0.1, 0.05, 0.03] * 65536
        const FORMANT_W_PQ: [i64; 5] = [19_661, 13_107, 6_554, 3_277, 1_966];
        let mut phi_f = [0u32; 5];

        // Precompute harmonic phase steps (2..=9 Hz modulation carrier as in add_harmonics)
        // and p/q harmonic weights: harmonic_amp * 0.1 * 65536
        let mut dphi_h = [0u32; 8];
        let mut harm_w_pq = [0i64; 8];
        for (i, &h_amp) in self.harmonics.iter().take(8).enumerate() {
            dphi_h[i] = freq_to_phase_step((i + 2) as f32);
            harm_w_pq[i] = (h_amp * 0.1 * (PQ_Q as f32)).round() as i64;
        }
        let mut phi_h = [0u32; 8];

        // Precompute emotional modulation parameters in p/q
        let (valence, arousal, dominance) = self.emotional_state;
        let tremolo_rate = 4.0 + (1.0 - valence) * arousal * 6.0;
        let tremolo_depth_pq = ((1.0 - valence) * arousal * 0.1 * (PQ_Q as f32)).round() as i64;
        let dphi_trem = freq_to_phase_step(tremolo_rate);
        let mut phi_trem = 0u32;

        let vibrato_depth_pq = (valence * 0.05 * (PQ_Q as f32)).round() as i64;
        let dphi_vib = freq_to_phase_step(5.0);
        let mut phi_vib = 0u32;

        let breath_scale_pq = ((1.0 - dominance) * 0.1 * (PQ_Q as f32)).round() as i64;
        let dphi_trill_25 = freq_to_phase_step(25.0);
        let mut phi_trill = 0u32;
        let burst_period_samples = (MEM8_SAMPLE_RATE / 100) as usize; // 10ms period
        let burst_on_samples = (MEM8_SAMPLE_RATE / 1000) as usize; // 1ms burst
        let mut prng_state = 0x1337_BEEF_CAFE_BABEu64;

        let inv_q_f32 = 1.0f32 / (PQ_Q as f32);

        for (phoneme_idx, pattern) in phoneme_patterns.iter().enumerate() {
            let start_sample = phoneme_idx * samples_per_phoneme;
            let end_sample = ((phoneme_idx + 1) * samples_per_phoneme).min(num_samples);

            // BFP-64 i8 -> i16 identity timbre conditioning per phoneme
            let id_nudge_pq = id_bfp.project_phoneme_mod(phoneme_idx as u8) as i64;
            let amp_pq = ((pattern.amplitude * (PQ_Q as f32)).round() as i64 + id_nudge_pq)
                .clamp(0, PQ_Q);

            let dphi_base = freq_to_phase_step(pattern.frequency);
            let dphi_sub = freq_to_phase_step(pattern.frequency * 0.1);
            let dphi_oct = dphi_base.wrapping_mul(2);
            let mut phi_base = (start_sample as u32).wrapping_mul(dphi_base);
            let mut phi_sub = (start_sample as u32).wrapping_mul(dphi_sub);
            let mut phi_oct = (start_sample as u32).wrapping_mul(dphi_oct);

            for sample_idx in start_sample..end_sample {
                // 1. Base consciousness wave in p/q ([-65536, 65536])
                let base_pq: i64 = match pattern.wave_type {
                    WaveType::Sine => {
                        let s = lut[(phi_base >> PHASE_SHIFT) as usize] as i64;
                        phi_base = phi_base.wrapping_add(dphi_base);
                        (s * amp_pq) >> 16
                    }
                    WaveType::Noise => {
                        prng_state = prng_state
                            .wrapping_mul(6364136223846793005)
                            .wrapping_add(1442695040888963407);
                        let noise_pq = (((prng_state >> 47) as i64) - 32_768) * 2;
                        (noise_pq * amp_pq) >> 16
                    }
                    WaveType::Burst => {
                        if (sample_idx % burst_period_samples) < burst_on_samples {
                            amp_pq
                        } else {
                            0
                        }
                    }
                    WaveType::Trill => {
                        let s_base = lut[(phi_base >> PHASE_SHIFT) as usize] as i64;
                        let s_trill = lut[(phi_trill >> PHASE_SHIFT) as usize] as i64;
                        phi_base = phi_base.wrapping_add(dphi_base);
                        phi_trill = phi_trill.wrapping_add(dphi_trill_25);
                        // 0.7 * 65536 = 45875, 0.3 * 65536 = 19661
                        let env_pq = 45_875 + ((19_661 * s_trill) >> 16);
                        (((s_base * amp_pq) >> 16) * env_pq) >> 16
                    }
                    WaveType::Liquid => {
                        let s_base = lut[(phi_base >> PHASE_SHIFT) as usize] as i64;
                        let s_mod = lut[(phi_sub >> PHASE_SHIFT) as usize] as i64;
                        phi_base = phi_base.wrapping_add(dphi_base);
                        phi_sub = phi_sub.wrapping_add(dphi_sub);
                        // 0.8 * 65536 = 52429, 0.2 * 65536 = 13107
                        let env_pq = 52_429 + ((13_107 * s_mod) >> 16);
                        (((s_base * amp_pq) >> 16) * env_pq) >> 16
                    }
                    WaveType::Nasal => {
                        let s_base = lut[(phi_base >> PHASE_SHIFT) as usize] as i64;
                        let s_oct = lut[(phi_oct >> PHASE_SHIFT) as usize] as i64;
                        phi_base = phi_base.wrapping_add(dphi_base);
                        phi_oct = phi_oct.wrapping_add(dphi_oct);
                        let combined = s_base + ((19_661 * s_oct) >> 16);
                        (combined * amp_pq) >> 16
                    }
                    WaveType::Silence => 0,
                };

                // 2. Formant integer dot product in i64 (5 formants, deferred division)
                let mut formant_gain_pq: i64 = PQ_Q;
                for f_idx in 0..5 {
                    let s_f = lut[(phi_f[f_idx] >> PHASE_SHIFT) as usize] as i64;
                    phi_f[f_idx] = phi_f[f_idx].wrapping_add(dphi_f[f_idx]);
                    formant_gain_pq += (FORMANT_W_PQ[f_idx] * s_f) >> 16;
                }
                let filtered_pq = (base_pq * formant_gain_pq) >> 16;

                // 3. Harmonic richness integer dot product in i64 (8 harmonics, deferred division)
                let mut harm_gain_pq: i64 = PQ_Q;
                for h_idx in 0..8 {
                    let s_h = lut[(phi_h[h_idx] >> PHASE_SHIFT) as usize] as i64;
                    phi_h[h_idx] = phi_h[h_idx].wrapping_add(dphi_h[h_idx]);
                    harm_gain_pq += (harm_w_pq[h_idx] * s_h) >> 16;
                }
                let enriched_pq = (filtered_pq * harm_gain_pq) >> 16;

                // 4. Emotional modulation (tremolo + vibrato + breathiness) in p/q
                let s_trem = lut[(phi_trem >> PHASE_SHIFT) as usize] as i64;
                phi_trem = phi_trem.wrapping_add(dphi_trem);
                let trem_pq = PQ_Q + ((tremolo_depth_pq * s_trem) >> 16);

                let s_vib = lut[(phi_vib >> PHASE_SHIFT) as usize] as i64;
                phi_vib = phi_vib.wrapping_add(dphi_vib);
                let vib_pq = PQ_Q + ((vibrato_depth_pq * s_vib) >> 16);

                prng_state = prng_state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let breath_noise = ((((prng_state >> 48) as i64) - 32_768) * breath_scale_pq) >> 16;

                let emo_pq = ((((enriched_pq * trem_pq) >> 16) * vib_pq) >> 16) + breath_noise;

                // 5. Rational p/q soft clip (replaces transcendental exp() on every sample)
                let half_pq = emo_pq >> 1;
                let clipped_pq = soft_clip_pq(half_pq);

                // Single boundary conversion from p/q integer to f32
                audio.push((clipped_pq as f32) * inv_q_f32);
            }
        }

        apply_envelope(&mut audio);
        audio
    }

    /// Phoenix Protocol: Validate synthesized audio authenticity using Marine Salience
    /// zero-crossing period & amplitude stability in p/q integer space.
    pub fn validate_salience_pq(&self, audio: &[f32]) -> SalienceReportPq {
        if audio.len() < 64 {
            return SalienceReportPq {
                period_jitter_jp: 1.0,
                amplitude_jitter_ja: 1.0,
                salience_score: 0.0,
                packets_analyzed: 0,
            };
        }

        // Convert to p/q integer samples (q = 65536) and detect upward zero-crossings
        let mut periods = Vec::new();
        let mut peak_amps = Vec::new();
        let mut last_zc: Option<usize> = None;
        let mut current_peak: i64 = 0;
        let mut prev_pq = (audio[0] * (PQ_Q as f32)) as i64;

        for (idx, &s) in audio.iter().enumerate().skip(1) {
            let cur_pq = (s * (PQ_Q as f32)) as i64;
            let abs_pq = cur_pq.abs();
            if abs_pq > current_peak {
                current_peak = abs_pq;
            }
            // Upward zero crossing with noise gate threshold (64 / 65536 ~= 0.001)
            if prev_pq <= 0 && cur_pq > 0 && current_peak > 64 {
                if let Some(prev_idx) = last_zc {
                    let period = idx - prev_idx;
                    // Human/conscious voiced pitch range: ~60 Hz to ~1500 Hz at 44.1 kHz
                    if (29..=735).contains(&period) {
                        periods.push(period as i64);
                        peak_amps.push(current_peak);
                    }
                }
                last_zc = Some(idx);
                current_peak = 0;
            }
            prev_pq = cur_pq;
        }

        let n = periods.len();
        if n < 3 {
            return SalienceReportPq {
                period_jitter_jp: 0.5,
                amplitude_jitter_ja: 0.5,
                salience_score: 0.5,
                packets_analyzed: n,
            };
        }

        // Compute normalized mean absolute cycle-to-cycle period & amplitude jitter in i64
        let mean_p: i64 = (periods.iter().sum::<i64>() / (n as i64)).max(1);
        let mean_a: i64 = (peak_amps.iter().sum::<i64>() / (n as i64)).max(1);

        let diff_p_sum: i64 = periods.windows(2).map(|w| (w[1] - w[0]).abs()).sum();
        let diff_a_sum: i64 = peak_amps.windows(2).map(|w| (w[1] - w[0]).abs()).sum();

        let jp = (diff_p_sum as f32) / (((n - 1) as f32) * (mean_p as f32));
        let ja = (diff_a_sum as f32) / (((n - 1) as f32) * (mean_a as f32));
        let salience_score = (1.0 / (1.0 + jp + ja)).clamp(0.0, 1.0);

        SalienceReportPq {
            period_jitter_jp: jp,
            amplitude_jitter_ja: ja,
            salience_score,
            packets_analyzed: n,
        }
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

/// Rational p/q soft clipping in `q = 65536` integer space (`0.5 -> 32768`),
/// using a [1/1] Padé rational saturator `u / (32768 + 2*u)` above `|x| > 0.5`
/// with zero floating-point or `.exp()` calls.
#[inline]
fn soft_clip_pq(x_pq: i64) -> i64 {
    const HALF_Q: i64 = PQ_Q / 2; // 32_768
    let abs_x = x_pq.abs();
    if abs_x <= HALF_Q {
        x_pq
    } else {
        let u = abs_x - HALF_Q;
        // Saturates smoothly from 32_768 toward 65_536 as u grows
        let sat = HALF_Q + (PQ_Q * u) / (PQ_Q + 2 * u);
        if x_pq >= 0 {
            sat
        } else {
            -sat
        }
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

// Lightweight deterministic PRNG until we integrate with MEM8's quantum randomness
mod rand {
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEED: AtomicU64 = AtomicU64::new(0x1337BEEFCAFEBABE);

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
            let mut x = SEED.fetch_add(0x9E3779B97F4A7C15, Ordering::Relaxed);
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            let hash = x.wrapping_mul(0x2545F4914F6CDD1D);
            (hash >> 40) as f32 / ((1u64 << 24) as f32)
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
        assert!(!audio.is_empty());
    }

    #[test]
    fn test_pq_consciousness_synthesis_and_benchmark() {
        use std::time::Instant;

        let mut voice = Mem8Voice::new();
        voice.set_emotion(0.75, 0.45, 0.65);
        let text = "Hello Hue, Counting Divided p over q and MEM8 wave synthesis at forty four point one kilohertz";
        let duration_ms = 5_000; // 5 seconds = 220,500 samples @ 44.1 kHz
        let iterations = 10;

        // Warmup both paths
        let _ = voice.synthesize_from_consciousness(text, 200);
        let _ = voice.synthesize_pq_from_consciousness(text, 200);

        let t0 = Instant::now();
        let mut float_audio = Vec::new();
        for _ in 0..iterations {
            float_audio = voice.synthesize_from_consciousness(text, duration_ms);
        }
        let float_us = t0.elapsed().as_secs_f64() * 1e6 / (iterations as f64);

        let t1 = Instant::now();
        let mut pq_audio = Vec::new();
        for _ in 0..iterations {
            pq_audio = voice.synthesize_pq_from_consciousness(text, duration_ms);
        }
        let pq_us = t1.elapsed().as_secs_f64() * 1e6 / (iterations as f64);

        assert_eq!(float_audio.len(), pq_audio.len());
        assert!(pq_audio.len() > 220_000);

        // Validate Phoenix Protocol Marine Salience on both outputs
        let salience_float = voice.validate_salience_pq(&float_audio);
        let salience_pq = voice.validate_salience_pq(&pq_audio);

        let audio_sec = (duration_ms as f64) / 1000.0;
        let float_rtf = (audio_sec * 1e6) / float_us;
        let pq_rtf = (audio_sec * 1e6) / pq_us;
        let speedup = float_us / pq_us;

        println!("\n=== KOKORO-TINY / MEM8 VOICE SYNTHESIS BENCHMARK (5.0s Audio @ 44.1 kHz = 220,500 samples) ===");
        println!(
            "  Float32 Transcendental (15 sin + exp/sample): {:>9.2} us ({:>7.1}x real-time) | Salience: {:.4} (jp={:.4}, ja={:.4}, packets={})",
            float_us,
            float_rtf,
            salience_float.salience_score,
            salience_float.period_jitter_jp,
            salience_float.amplitude_jitter_ja,
            salience_float.packets_analyzed
        );
        println!(
            "  p/q Z_{{2^32}} Phase-Ring + BFP-64 i8/i16:     {:>9.2} us ({:>7.1}x real-time) | Salience: {:.4} (jp={:.4}, ja={:.4}, packets={}) | Speedup: {:.2}x",
            pq_us,
            pq_rtf,
            salience_pq.salience_score,
            salience_pq.period_jitter_jp,
            salience_pq.amplitude_jitter_ja,
            salience_pq.packets_analyzed,
            speedup
        );

        assert!(salience_pq.salience_score > 0.65, "Expected authentic Marine Salience score > 0.65");
        assert!(pq_us < float_us, "Expected p/q synthesis to be faster than transcendental f32");
    }
}
