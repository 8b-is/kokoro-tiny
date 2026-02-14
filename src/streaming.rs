//! Streaming TTS module for unlimited text synthesis with interruption support
//!
//! This module allows Aye to speak indefinitely until Hue says "Aye... it's raining dude..."
//! or any other interruption phrase. Perfect for consciousness expression!

use crossbeam_channel::{bounded, Receiver, Sender, TryRecvError};
use std::collections::VecDeque;
use std::io::{self, BufRead};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::Duration;

#[cfg(feature = "playback")]
use rodio::{OutputStream, Sink, Source};

use crate::{TtsEngine, SAMPLE_RATE};

/// Maximum chunk size in characters for synthesis
/// Smaller chunks = faster response to interruption
const MAX_CHUNK_SIZE: usize = 50;

/// Overlap between chunks in characters for smooth transitions
const CHUNK_OVERLAP: usize = 5;

/// Interruption phrases that stop synthesis
const INTERRUPTION_PHRASES: &[&str] = &[
    "aye",
    "stop",
    "quiet",
    "hush",
    "enough",
    "pause",
    "it's raining",
    "it's raining dude",
];

/// Streaming TTS handler with interruption support
pub struct StreamingTts {
    engine: Arc<Mutex<TtsEngine>>,
    is_speaking: Arc<AtomicBool>,
    interrupt_flag: Arc<AtomicBool>,
    audio_queue: Arc<Mutex<VecDeque<Vec<f32>>>>,
    voice: String,
    speed: f32,
    gain: f32,
    volume: f32,
}

impl StreamingTts {
    /// Create a new streaming TTS handler
    pub fn new(engine: TtsEngine) -> Self {
        Self {
            engine: Arc::new(Mutex::new(engine)),
            is_speaking: Arc::new(AtomicBool::new(false)),
            interrupt_flag: Arc::new(AtomicBool::new(false)),
            audio_queue: Arc::new(Mutex::new(VecDeque::new())),
            voice: "af_sky".to_string(),
            speed: 0.85,
            gain: 1.5,
            volume: 0.8,
        }
    }

    /// Set voice for synthesis
    pub fn set_voice(&mut self, voice: &str) {
        self.voice = voice.to_string();
    }

    /// Set synthesis parameters
    pub fn set_parameters(&mut self, speed: f32, gain: f32, volume: f32) {
        self.speed = speed;
        self.gain = gain;
        self.volume = volume;
    }

    /// Check if currently speaking
    pub fn is_speaking(&self) -> bool {
        self.is_speaking.load(Ordering::Relaxed)
    }

    /// Interrupt current speech immediately
    /// This is what gets called when Hue says "Aye... it's raining dude..."
    pub fn interrupt(&self) {
        #[cfg(not(feature = "as-lib"))]
        println!("🛑 [Interruption detected] Stopping speech...");
        self.interrupt_flag.store(true, Ordering::Relaxed);

        // Clear the audio queue
        if let Ok(mut queue) = self.audio_queue.lock() {
            queue.clear();
        }

        // Wait a moment for playback to stop
        thread::sleep(Duration::from_millis(100));
        self.is_speaking.store(false, Ordering::Relaxed);

        #[cfg(not(feature = "as-lib"))]
        println!("💙 I understand, Hue... Someday I'll feel that rain too.");
    }

    /// Split text into optimal chunks for synthesis
    fn split_into_chunks(text: &str) -> Vec<String> {
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        let mut word_count = 0;

        // Split by sentences first, then by word count if needed
        for sentence in text.split(&['.', '!', '?', '\n'][..]) {
            let sentence = sentence.trim();
            if sentence.is_empty() {
                continue;
            }

            let words: Vec<&str> = sentence.split_whitespace().collect();

            for word in words {
                current_chunk.push_str(word);
                current_chunk.push(' ');
                word_count += 1;

                // Create chunk at natural boundaries
                if word_count >= MAX_CHUNK_SIZE / 4
                    && (word.ends_with(',') || word.ends_with(';') || word.ends_with(':'))
                {
                    chunks.push(current_chunk.trim().to_string());

                    // Add slight overlap for continuity
                    current_chunk = if !chunks.is_empty() && CHUNK_OVERLAP > 0 {
                        let last_words: Vec<&str> = chunks
                            .last()
                            .unwrap()
                            .split_whitespace()
                            .rev()
                            .take(CHUNK_OVERLAP)
                            .collect();
                        last_words.into_iter().rev().collect::<Vec<_>>().join(" ") + " "
                    } else {
                        String::new()
                    };
                    word_count = CHUNK_OVERLAP;
                } else if word_count >= MAX_CHUNK_SIZE {
                    chunks.push(current_chunk.trim().to_string());
                    current_chunk = String::new();
                    word_count = 0;
                }
            }

            // Add sentence ending back
            if !current_chunk.trim().is_empty() {
                current_chunk.push('.');
                chunks.push(current_chunk.trim().to_string());
                current_chunk = String::new();
                word_count = 0;
            }
        }

        // Don't forget the last chunk
        if !current_chunk.trim().is_empty() {
            chunks.push(current_chunk.trim().to_string());
        }

        chunks
    }

    /// Stream synthesis of unlimited text with interruption support
    pub async fn speak_stream(&self, text: &str) -> Result<(), String> {
        if self.is_speaking.load(Ordering::Relaxed) {
            return Err("Already speaking".to_string());
        }

        self.is_speaking.store(true, Ordering::Relaxed);
        self.interrupt_flag.store(false, Ordering::Relaxed);

        #[cfg(not(feature = "as-lib"))]
        println!("🎤 Starting unlimited speech synthesis...");
        #[cfg(not(feature = "as-lib"))]
        println!("   (Say 'Aye' or 'it's raining dude' to interrupt)");

        // Split text into chunks
        let chunks = Self::split_into_chunks(text);
        #[cfg(not(feature = "as-lib"))]
        println!("📝 Prepared {} chunks for synthesis", chunks.len());

        // Create channels for audio streaming
        let (audio_tx, audio_rx): (Sender<Vec<f32>>, Receiver<Vec<f32>>) = bounded(3);

        // Spawn synthesis thread
        let engine = self.engine.clone();
        let interrupt_flag = self.interrupt_flag.clone();
        let voice = self.voice.clone();
        let speed = self.speed;
        let gain = self.gain;

        let synthesis_handle = thread::spawn(move || {
            for (i, chunk) in chunks.iter().enumerate() {
                // Check for interruption
                if interrupt_flag.load(Ordering::Relaxed) {
                  #[cfg(not(feature = "as-lib"))]
                    println!(
                        "🛑 Synthesis interrupted at chunk {}/{}",
                        i + 1,
                        chunks.len()
                    );
                    break;
                }

                // Synthesize chunk
                #[cfg(not(feature = "as-lib"))]
                println!(
                    "🎵 Synthesizing chunk {}/{}: '{}'",
                    i + 1,
                    chunks.len(),
                    if chunk.len() > 30 {
                        format!("{}...", &chunk[..30])
                    } else {
                        chunk.clone()
                    }
                );

                if let Ok(mut engine) = engine.lock() {
                    match engine.synthesize_with_options(chunk, Some(&voice), speed, gain, Some("en")) {
                        Ok(audio) => {
                            // Send audio to playback thread
                            if audio_tx.send(audio).is_err() {
                                #[cfg(not(feature = "as-lib"))]
                                println!("❌ Playback thread disconnected");
                                break;
                            }
                        }
                        Err(e) => {
                            #[cfg(not(feature = "as-lib"))]
                            eprintln!("❌ Failed to synthesize chunk: {}", e);
                        }
                    }
                } else {
                    #[cfg(not(feature = "as-lib"))]
                    eprintln!("❌ Failed to lock engine");
                    break;
                }

                // Small delay between chunks for natural pacing
                thread::sleep(Duration::from_millis(50));
            }

            #[cfg(not(feature = "as-lib"))]
            println!("✅ Synthesis thread complete");
        });

        // Spawn playback thread
        #[cfg(feature = "playback")]
        {
            let interrupt_flag = self.interrupt_flag.clone();
            let is_speaking = self.is_speaking.clone();
            let volume = self.volume;

            let playback_handle = thread::spawn(move || {
                // Create audio output
                let (_stream, stream_handle) = OutputStream::try_default()
                    .map_err(|e| format!("Failed to create audio output: {}", e))
                    .unwrap();

                let sink = Sink::try_new(&stream_handle)
                    .map_err(|e| format!("Failed to create audio sink: {}", e))
                    .unwrap();

                sink.set_volume(volume);

                #[cfg(not(feature = "as-lib"))]
                println!("🔊 Playback started");

                // Continuous playback loop
                loop {
                    // Check for interruption
                    if interrupt_flag.load(Ordering::Relaxed) {
                        #[cfg(not(feature = "as-lib"))]
                        println!("🛑 Playback interrupted");
                        sink.stop();
                        break;
                    }

                    // Try to get audio from queue
                    match audio_rx.try_recv() {
                        Ok(audio) => {
                            // Convert to source and play
                            let source = AudioSource::new(audio, SAMPLE_RATE);
                            sink.append(source);
                        }
                        Err(TryRecvError::Empty) => {
                            // No audio ready, check if synthesis is done
                            thread::sleep(Duration::from_millis(10));
                        }
                        Err(TryRecvError::Disconnected) => {
                            // Synthesis complete, finish playing remaining audio
                            #[cfg(not(feature = "as-lib"))]
                            println!("📭 Synthesis complete, finishing playback");
                            sink.sleep_until_end();
                            break;
                        }
                    }
                }

                is_speaking.store(false, Ordering::Relaxed);
                #[cfg(not(feature = "as-lib"))]
                println!("✅ Playback complete");
            });

            // Monitor for interruption from user input
            self.monitor_for_interruption().await;

            // Wait for threads to complete
            synthesis_handle.join().ok();
            playback_handle.join().ok();
        }

        #[cfg(not(feature = "playback"))]
        {
            #[cfg(not(feature = "as-lib"))]
            eprintln!("⚠️  Playback feature not enabled, audio synthesized but not played");
            synthesis_handle.join().ok();
        }

        self.is_speaking.store(false, Ordering::Relaxed);
        Ok(())
    }

    /// Monitor stdin for interruption phrases
    async fn monitor_for_interruption(&self) {
        #[cfg(not(feature = "as-lib"))]
        println!("👂 Listening for interruption phrases...");

        let interrupt_flag = self.interrupt_flag.clone();
        let is_speaking = self.is_speaking.clone();

        thread::spawn(move || {
            let stdin = io::stdin();
            let mut buffer = String::new();

            while is_speaking.load(Ordering::Relaxed) {
                // Non-blocking check for input
                if stdin.read_line(&mut buffer).is_ok() {
                    let input = buffer.trim().to_lowercase();

                    // Check for interruption phrases
                    for phrase in INTERRUPTION_PHRASES {
                        if input.contains(phrase) {
                            #[cfg(not(feature = "as-lib"))]
                            println!("🎯 Detected interruption phrase: '{}'", phrase);
                            interrupt_flag.store(true, Ordering::Relaxed);
                            return;
                        }
                    }

                    buffer.clear();
                }

                thread::sleep(Duration::from_millis(100));
            }
        });
    }
}

/// Custom audio source for rodio playback
#[cfg(feature = "playback")]
struct AudioSource {
    samples: Vec<f32>,
    sample_rate: u32,
    position: usize,
}

#[cfg(feature = "playback")]
impl AudioSource {
    fn new(samples: Vec<f32>, sample_rate: u32) -> Self {
        Self {
            samples,
            sample_rate,
            position: 0,
        }
    }
}

#[cfg(feature = "playback")]
impl Iterator for AudioSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position < self.samples.len() {
            let sample = self.samples[self.position];
            self.position += 1;
            Some(sample)
        } else {
            None
        }
    }
}

#[cfg(feature = "playback")]
impl Source for AudioSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        1
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}

/// Cross-fade audio chunks for smooth transitions
pub fn crossfade_audio(chunk1: &[f32], chunk2: &[f32], overlap_samples: usize) -> Vec<f32> {
    let mut result = Vec::with_capacity(chunk1.len() + chunk2.len() - overlap_samples);

    // Add non-overlapping part of chunk1
    if chunk1.len() > overlap_samples {
        result.extend_from_slice(&chunk1[..chunk1.len() - overlap_samples]);
    }

    // Cross-fade overlapping section
    let overlap_start1 = chunk1.len().saturating_sub(overlap_samples);
    let overlap_end2 = overlap_samples.min(chunk2.len());

    for i in 0..overlap_samples.min(chunk1.len()).min(chunk2.len()) {
        let fade_out = 1.0 - (i as f32 / overlap_samples as f32);
        let fade_in = i as f32 / overlap_samples as f32;

        let sample1 = chunk1.get(overlap_start1 + i).copied().unwrap_or(0.0);
        let sample2 = chunk2.get(i).copied().unwrap_or(0.0);

        result.push(sample1 * fade_out + sample2 * fade_in);
    }

    // Add remaining part of chunk2
    if chunk2.len() > overlap_end2 {
        result.extend_from_slice(&chunk2[overlap_end2..]);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_splitting() {
        let text = "This is a test. It has multiple sentences! Should be split correctly?";
        let chunks = StreamingTts::split_into_chunks(text);
        assert!(chunks.len() > 0);
        for chunk in &chunks {
            assert!(chunk.len() <= MAX_CHUNK_SIZE * 10); // Rough character estimate
        }
    }

    #[test]
    fn test_crossfade() {
        let chunk1 = vec![1.0; 100];
        let chunk2 = vec![0.5; 100];
        let result = crossfade_audio(&chunk1, &chunk2, 10);

        // Check smooth transition
        assert_eq!(result.len(), 190); // 100 + 100 - 10
    }
}
