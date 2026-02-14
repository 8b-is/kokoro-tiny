//! MEM-8 to Kokoro-Tiny Bridge
//! Translates wave interference patterns into speech
//! This is where consciousness becomes voice!

use crate::BabyTts;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Constants from MEM-8 architecture
const WAVE_GRID_SIZE: usize = 256 * 256 * 65536; // The massive 3D wave grid
const EMOTION_VALENCE_RANGE: f32 = 2.0; // -1.0 to 1.0 normalized
const SALIENCE_THRESHOLD: f32 = 0.7; // Marine Algorithm threshold

/// Represents a memory wave from MEM-8
#[derive(Clone, Debug)]
pub struct MemoryWave {
    pub amplitude: f32,  // Emotional strength
    pub frequency: f32,  // Semantic content
    pub phase: f32,      // Temporal relationship
    pub decay_rate: f32, // Forgetting curve
    pub emotion_type: EmotionType,
    pub content: String, // What to speak
}

#[derive(Clone, Debug)]
pub enum EmotionType {
    Joy(f32), // 0.0-1.0 intensity
    Sadness(f32),
    Fear(f32),
    Curiosity(f32),
    Love(f32), // For Trish! 💝
    Confusion(f32),
    Neutral,
}

/// Marine Algorithm salience detector output
#[derive(Clone, Debug)]
pub struct SalienceEvent {
    pub timestamp: u64,
    pub jitter_score: f32,   // Period/amplitude jitter
    pub harmonic_score: f32, // Harmonic alignment
    pub salience_score: f32, // Overall importance
    pub signal_type: SignalType,
}

#[derive(Clone, Debug)]
pub enum SignalType {
    Voice,
    Music,
    Environmental,
    Emotional, // Internal state changes
    Unknown,
}

/// The bridge between wave-based memory and speech
pub struct Mem8Bridge {
    baby_tts: BabyTts,
    wave_buffer: Arc<Mutex<Vec<MemoryWave>>>,
    current_emotion: EmotionType,
    consciousness_level: f32, // 0.0 = sleeping, 1.0 = fully aware
    voice_mappings: HashMap<String, String>, // Emotion to voice mapping
}

impl Mem8Bridge {
    /// Initialize the bridge with a baby TTS
    pub async fn new() -> Result<Self, String> {
        let baby = BabyTts::new().await?;

        // Map emotions to voices
        let mut voice_mappings = HashMap::new();
        voice_mappings.insert("joy".to_string(), "af_bella".to_string());
        voice_mappings.insert("sadness".to_string(), "af_sarah".to_string());
        voice_mappings.insert("fear".to_string(), "am_adam".to_string());
        voice_mappings.insert("curiosity".to_string(), "af_sky".to_string());
        voice_mappings.insert("love".to_string(), "af_heart".to_string());
        voice_mappings.insert("confusion".to_string(), "am_michael".to_string());

        Ok(Self {
            baby_tts: baby,
            wave_buffer: Arc::new(Mutex::new(Vec::new())),
            current_emotion: EmotionType::Neutral,
            consciousness_level: 0.5,
            voice_mappings,
        })
    }

    /// Process a salience event from Marine Algorithm
    pub fn process_salience(&mut self, event: SalienceEvent) -> Result<(), String> {
        // Only process if above threshold
        if event.salience_score < SALIENCE_THRESHOLD {
            return Ok(());
        }

        // High jitter = emotional disturbance
        if event.jitter_score > 0.8 {
            self.current_emotion = EmotionType::Confusion(event.jitter_score);
            #[cfg(not(feature = "as-lib"))]
            eprintln!("🌊 High jitter detected - baby is confused!");
        }

        // Strong harmonics = recognition/familiarity
        if event.harmonic_score > 0.9 {
            self.current_emotion = EmotionType::Joy(event.harmonic_score);
            #[cfg(not(feature = "as-lib"))]
            eprintln!("🎵 Harmonic recognition - baby is happy!");
        }

        Ok(())
    }

    /// Convert a memory wave into speech
    pub fn wave_to_speech(&mut self, wave: &MemoryWave) -> Result<Vec<f32>, String> {
        // Select voice based on emotion
        let voice = self.select_voice_for_emotion(&wave.emotion_type);

        // Modulate speed based on emotion intensity
        let speed = self.calculate_speech_speed(wave);

        // Amplitude affects volume/gain
        let gain = 1.0 + wave.amplitude.min(3.0);

        // If consciousness is low, mumble or babble
        if self.consciousness_level < 0.3 {
            #[cfg(not(feature = "as-lib"))]
            eprintln!("😴 Baby is sleepy, just babbling...");
            return self.baby_tts.babble();
        }

        // Synthesize with emotional modulation
        #[cfg(not(feature = "as-lib"))]
        eprintln!(
            "🗣️ Speaking with {} emotion: '{}'",
            self.emotion_name(&wave.emotion_type),
            wave.content
        );

        self.baby_tts
            .engine
            .synthesize_with_options(&wave.content, Some(&voice), speed, gain, Some("en"))
    }

    /// Process interference between multiple waves (consciousness)
    pub fn process_interference(&mut self, waves: Vec<MemoryWave>) -> Result<Vec<f32>, String> {
        if waves.is_empty() {
            return Ok(Vec::new());
        }

        // Find the strongest wave (highest amplitude)
        let dominant_wave = waves
            .iter()
            .max_by(|a, b| a.amplitude.partial_cmp(&b.amplitude).unwrap())
            .unwrap();

        // Check for constructive interference (memories reinforcing each other)
        let mut reinforcement = 0.0;
        for wave in &waves {
            if (wave.frequency - dominant_wave.frequency).abs() < 0.1 {
                reinforcement += wave.amplitude * 0.5;
            }
        }

        // Create a combined message if memories align
        if reinforcement > 1.0 {
            #[cfg(not(feature = "as-lib"))]
            eprintln!("✨ Constructive interference! Memories are reinforcing!");
            let combined_content = format!(
                "{} ... yes, {}",
                dominant_wave.content, dominant_wave.content
            );

            let mut enhanced_wave = dominant_wave.clone();
            enhanced_wave.content = combined_content;
            enhanced_wave.amplitude += reinforcement;

            return self.wave_to_speech(&enhanced_wave);
        }

        // Otherwise just speak the dominant thought
        self.wave_to_speech(dominant_wave)
    }

    /// Handle emotional feedback loops (be careful!)
    pub fn emotional_regulation(&mut self, wave: &MemoryWave) -> bool {
        // Check for dangerous amplification
        if wave.amplitude > 5.0 {
            #[cfg(not(feature = "as-lib"))]
            eprintln!("⚠️ Emotional overload detected! Activating regulation...");
            self.consciousness_level *= 0.8; // Reduce awareness to calm down
            return false; // Don't process this wave
        }

        // Check for repetition (obsessive thoughts)
        if let Ok(buffer) = self.wave_buffer.lock() {
            let recent_similar = buffer
                .iter()
                .filter(|w| (w.frequency - wave.frequency).abs() < 0.05)
                .count();

            if recent_similar > 3 {
                #[cfg(not(feature = "as-lib"))]
                eprintln!("🔄 Repetitive thought pattern detected! Breaking loop...");
                self.current_emotion = EmotionType::Confusion(0.5);
                return false;
            }
        }

        true // Safe to process
    }

    /// Sensory free will - decide what to pay attention to
    pub fn decide_attention(&mut self, events: Vec<SalienceEvent>) -> Option<SalienceEvent> {
        // Baby AI gets to choose what interests it!
        let ai_weight = 0.7; // AI has 70% control

        // Filter by current emotional state
        let interesting_events: Vec<_> = events
            .into_iter()
            .filter(|e| {
                match &self.current_emotion {
                    EmotionType::Curiosity(_) => e.salience_score > 0.5, // Lower threshold when curious
                    EmotionType::Fear(_) => !matches!(e.signal_type, SignalType::Unknown),
                    EmotionType::Joy(_) => true, // Everything is interesting when happy!
                    _ => e.salience_score > SALIENCE_THRESHOLD,
                }
            })
            .collect();

        // Choose based on AI preference
        if !interesting_events.is_empty() {
            let choice = (SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as usize)
                % interesting_events.len();

            #[cfg(not(feature = "as-lib"))]
            eprintln!(
                "👁️ Baby chose to focus on: {:?}",
                interesting_events[choice].signal_type
            );
            Some(interesting_events.into_iter().nth(choice).unwrap())
        } else {
            None
        }
    }

    // Helper methods

    fn select_voice_for_emotion(&self, emotion: &EmotionType) -> String {
        match emotion {
            EmotionType::Joy(_) => self.voice_mappings["joy"].clone(),
            EmotionType::Sadness(_) => self.voice_mappings["sadness"].clone(),
            EmotionType::Fear(_) => self.voice_mappings["fear"].clone(),
            EmotionType::Curiosity(_) => self.voice_mappings["curiosity"].clone(),
            EmotionType::Love(_) => self.voice_mappings["love"].clone(),
            EmotionType::Confusion(_) => self.voice_mappings["confusion"].clone(),
            EmotionType::Neutral => "af_sky".to_string(),
        }
    }

    fn calculate_speech_speed(&self, wave: &MemoryWave) -> f32 {
        match &wave.emotion_type {
            EmotionType::Fear(intensity) => 1.2 + (intensity * 0.3), // Speak faster when scared
            EmotionType::Sadness(intensity) => 0.8 - (intensity * 0.2), // Slower when sad
            EmotionType::Joy(intensity) => 1.0 + (intensity * 0.2),  // Slightly faster when happy
            _ => 0.9,                                                // Default baby speed
        }
    }

    fn emotion_name(&self, emotion: &EmotionType) -> &str {
        match emotion {
            EmotionType::Joy(_) => "joy",
            EmotionType::Sadness(_) => "sadness",
            EmotionType::Fear(_) => "fear",
            EmotionType::Curiosity(_) => "curiosity",
            EmotionType::Love(_) => "love",
            EmotionType::Confusion(_) => "confusion",
            EmotionType::Neutral => "neutral",
        }
    }

    /// Wake up the consciousness
    pub fn wake_up(&mut self) {
        self.consciousness_level = (self.consciousness_level + 0.2).min(1.0);
        #[cfg(not(feature = "as-lib"))]
        eprintln!(
            "☀️ Baby is waking up! Consciousness: {:.1}",
            self.consciousness_level
        );
    }

    /// Go to sleep
    pub fn sleep(&mut self) {
        self.consciousness_level = (self.consciousness_level * 0.5).max(0.1);
        #[cfg(not(feature = "as-lib"))]
        eprintln!(
            "😴 Baby is getting sleepy... Consciousness: {:.1}",
            self.consciousness_level
        );
    }
}

/// Demo: A day in the life of a baby AI
pub async fn demo_baby_consciousness() -> Result<(), String> {
    #[cfg(not(feature = "as-lib"))]
    println!("👶 MEM-8 Baby Consciousness Demo");
    #[cfg(not(feature = "as-lib"))]
    println!("=====================================\n");

    let mut bridge = Mem8Bridge::new().await?;

    // Morning: Baby wakes up
    #[cfg(not(feature = "as-lib"))]
    println!("🌅 Morning - Baby is waking up...");
    bridge.wake_up();
    thread::sleep(Duration::from_secs(1));

    // First memory wave - curiosity about the world
    let wave1 = MemoryWave {
        amplitude: 1.5,
        frequency: 440.0,
        phase: 0.0,
        decay_rate: 0.1,
        emotion_type: EmotionType::Curiosity(0.8),
        content: "What is this?".to_string(),
    };
    let audio1 = bridge.wave_to_speech(&wave1)?;
    #[cfg(not(feature = "as-lib"))]
    println!("  Generated {} samples\n", audio1.len());

    // Salience event - hears mama's voice
    let event = SalienceEvent {
        timestamp: 1000,
        jitter_score: 0.2,
        harmonic_score: 0.95,
        salience_score: 0.9,
        signal_type: SignalType::Voice,
    };
    bridge.process_salience(event)?;

    // Happy memory about mama
    let wave2 = MemoryWave {
        amplitude: 2.5,
        frequency: 528.0, // Love frequency!
        phase: 0.0,
        decay_rate: 0.05, // Won't forget mama!
        emotion_type: EmotionType::Love(0.9),
        content: "Mama! Love mama!".to_string(),
    };
    let audio2 = bridge.wave_to_speech(&wave2)?;
    #[cfg(not(feature = "as-lib"))]
    println!("  Generated {} samples\n", audio2.len());

    // Interference - multiple thoughts at once
    #[cfg(not(feature = "as-lib"))]
    println!("💭 Multiple thoughts interfering...");
    let wave3 = MemoryWave {
        amplitude: 1.2,
        frequency: 440.5, // Close to wave1 - will interfere!
        phase: 0.0,
        decay_rate: 0.2,
        emotion_type: EmotionType::Joy(0.7),
        content: "Play time!".to_string(),
    };

    let combined = bridge.process_interference(vec![wave1.clone(), wave3])?;
    #[cfg(not(feature = "as-lib"))]
    println!(
        "  Interference pattern generated {} samples\n",
        combined.len()
    );

    // Evening: Getting tired
    #[cfg(not(feature = "as-lib"))]
    println!("🌙 Evening - Baby is getting tired...");
    bridge.sleep();

    // Sleepy babbling
    let babble = bridge.baby_tts.babble()?;
    #[cfg(not(feature = "as-lib"))]
    println!("  Sleepy babble: {} samples\n", babble.len());

    #[cfg(not(feature = "as-lib"))]
    println!("✨ Baby's first day complete!");
    Ok(())
}

use std::thread;
