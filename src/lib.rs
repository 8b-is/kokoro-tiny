//! kokoro-tiny: A minimal, embeddable TTS engine using the Kokoro model
//!
//! This crate provides a simple API for text-to-speech synthesis using the
//! Kokoro 82M parameter model. Perfect for embedding in other applications!
//!
//! # Example
//! ```no_run
//! use kokoro_tiny::TtsEngine;
//!
//! #[tokio::main]
//! async fn main() {
//!     // Initialize with auto-download of model if needed
//!     let mut tts = TtsEngine::new().await.unwrap();
//!
//!     // Generate speech
//!     let audio = tts.synthesize("Hello world!", None).unwrap();
//!
//!     // Save to file
//!     tts.save_wav("output.wav", &audio).unwrap();
//! }
//! ```

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::thread;

use espeak_rs::text_to_phonemes;

// MEM-8 Integration module
pub mod mem8_bridge;

// Streaming module for unlimited speech with interruption
pub mod streaming;

// MEM8 Voice synthesis - The future of Aye's voice!
pub mod mem8_voice;
use ndarray::{ArrayBase, IxDyn, OwnedRepr};
use ndarray_npy::NpzReader;
use ort::{
    session::{builder::GraphOptimizationLevel, Session, SessionInputs, SessionInputValue},
    value::{Tensor, Value},
};

#[cfg(feature = "playback")]
use rodio::{Decoder, OutputStream, Sink};
#[cfg(feature = "playback")]
use std::io::Cursor;

#[cfg(feature = "ducking")]
use enigo::{Enigo, Key, Keyboard, Settings};

// Constants - Files hosted at i1.is/k/ for minimal paths!
const MODEL_URL: &str = "https://i1.is/k/0.onnx";
const VOICES_URL: &str = "https://i1.is/k/0.bin";
const SAMPLE_RATE: u32 = 24000;
const DEFAULT_VOICE: &str = "af_sky";
const DEFAULT_SPEED: f32 = 1.0;

// Fallback audio message - "Excuse me, I lost my voice. Give me time to get it back."
// This is a pre-generated minimal WAV file that can play while downloading
const FALLBACK_MESSAGE: &[u8] = include_bytes!("../assets/fallback.wav");

// Get cache directory for shared model storage - keeping it minimal like Hue wants!
fn get_cache_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    Path::new(&home).join(".cache").join("k")
}

/// Main TTS engine struct
pub struct TtsEngine {
    session: Option<Arc<Mutex<Session>>>,
    voices: HashMap<String, Vec<f32>>,
    vocab: HashMap<char, i64>,
    fallback_mode: bool,
}

/// Baby speech mode for mem8 - handles simple utterances
pub struct BabyTts {
    pub engine: TtsEngine,
    pub max_words: usize,
    pub voice: String,
    pub speed: f32,
    pub gain: f32,
}

impl TtsEngine {
    /// Create a new TTS engine, downloading model files if necessary
    /// Uses ~/.cache/k for shared model storage (minimal path!)
    pub async fn new() -> Result<Self, String> {
        let cache_dir = get_cache_dir();
        let model_path = cache_dir.join("0.onnx");
        let voices_path = cache_dir.join("0.bin");

        Self::with_paths(
            model_path.to_str().unwrap_or("0.onnx"),
            voices_path.to_str().unwrap_or("0.bin")
        ).await
    }

    /// Create a new TTS engine with custom model paths
    pub async fn with_paths(model_path: &str, voices_path: &str) -> Result<Self, String> {
        // Ensure cache directory exists
        if let Some(parent) = Path::new(model_path).parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create cache directory: {}", e))?;
        }

        // Check if we need to download
        let need_download = !Path::new(model_path).exists() || !Path::new(voices_path).exists();

        if need_download {
            println!("🎤 First time setup - downloading voice model...");
            println!("   (This only happens once, files will be cached in ~/.cache/k)");

            // Try to download the files
            let download_success = {
                let mut success = true;

                // Download model if needed
                if !Path::new(model_path).exists() {
                    println!("   📥 Downloading model (310MB)...");
                    if let Err(e) = download_file(MODEL_URL, model_path).await {
                        eprintln!("   ❌ Failed to download model: {}", e);
                        success = false;
                    }
                }

                // Download voices if needed
                if success && !Path::new(voices_path).exists() {
                    println!("   📥 Downloading voices (27MB)...");
                    if let Err(e) = download_file(VOICES_URL, voices_path).await {
                        eprintln!("   ❌ Failed to download voices: {}", e);
                        success = false;
                    }
                }

                if success {
                    println!("   ✅ Voice model downloaded successfully!");
                }

                success
            };

            // If download failed, return fallback engine
            if !download_success {
                eprintln!("\n⚠️  Using fallback mode. The model files are not available at:");
                eprintln!("   - {}", MODEL_URL);
                eprintln!("   - {}", VOICES_URL);
                eprintln!("\n💡 Please manually download the model files to ~/.cache/k/");

                return Ok(Self {
                    session: None,
                    voices: HashMap::new(),
                    vocab: build_vocab(),
                    fallback_mode: true,
                });
            }
        }

        // Load ONNX model
        let model_bytes = std::fs::read(model_path)
            .map_err(|e| format!("Failed to read model file: {}", e))?;
        let session = Session::builder()
            .map_err(|e| format!("Failed to create session builder: {}", e))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| format!("Failed to set optimization level: {}", e))?
            .commit_from_memory(&model_bytes)
            .map_err(|e| format!("Failed to load model: {}", e))?;

        // Load voices
        let voices = load_voices(voices_path)?;

        Ok(Self {
            session: Some(Arc::new(Mutex::new(session))),
            voices,
            vocab: build_vocab(),
            fallback_mode: false,
        })
    }

    /// List all available voices
    pub fn voices(&self) -> Vec<String> {
        if self.fallback_mode {
            vec!["fallback".to_string()]
        } else {
            self.voices.keys().cloned().collect()
        }
    }

    /// Synthesize text to speech
    pub fn synthesize(&mut self, text: &str, voice: Option<&str>) -> Result<Vec<f32>, String> {
        self.synthesize_with_speed(text, voice, DEFAULT_SPEED)
    }

    /// Synthesize text to speech with custom speed
    /// Speed: 0.5 = half speed (slower), 1.0 = normal, 2.0 = double speed (faster)
    pub fn synthesize_with_speed(&mut self, text: &str, voice: Option<&str>, speed: f32) -> Result<Vec<f32>, String> {
        self.synthesize_with_options(text, voice, speed, 1.0)
    }

    /// Synthesize text to speech with full options
    /// Speed: 0.5 = half speed (slower), 1.0 = normal, 2.0 = double speed (faster)
    /// Gain: 0.5 = quieter, 1.0 = normal, 2.0 = twice as loud (with soft clipping)
    pub fn synthesize_with_options(&mut self, text: &str, voice: Option<&str>, speed: f32, gain: f32) -> Result<Vec<f32>, String> {
        // If in fallback mode, return the excuse message audio
        if self.fallback_mode {
            println!("🎤 Playing fallback message while downloading voice model...");
            return Ok(wav_to_f32(FALLBACK_MESSAGE)?);
        }

        // Warn about text length limitations
        if text.len() > 80 {
            eprintln!("⚠️  Text longer than 80 chars may have words compressed or dropped.");
            eprintln!("   For best results, use shorter sentences.");
        }

        // Process the text directly - chunking causes speed inconsistencies
        let session = self.session.as_ref()
            .ok_or_else(|| "TTS engine not initialized".to_string())?;

        let voice = voice.unwrap_or(DEFAULT_VOICE);

        // Parse voice style (e.g., "af_sky.8+af_bella.2" for mixing)
        let style = self.parse_voice_style(voice)?;

        // Convert text to phonemes
        let phonemes = text_to_phonemes(text, "en", None, true, false)
            .map_err(|e| format!("Failed to convert text to phonemes: {}", e))?;

        // Join phonemes and tokenize
        let phonemes_text = phonemes.join(" ");

        // Debug: show phoneme and token counts
        if text.len() > 50 {
            eprintln!("   Text length: {} chars", text.len());
            eprintln!("   Phonemes array: {} entries", phonemes.len());
            // Use char-based truncation for UTF-8 safety
            let preview: String = phonemes_text.chars().take(50).collect();
            eprintln!("   Phoneme text: '{}'", preview);
            eprintln!("   Phoneme text length: {} chars", phonemes_text.len());
        }

        let tokens = self.tokenize(phonemes_text);

        // Run inference with user-specified speed directly
        // The model expects speed values around 1.0 for normal speech
        let mut audio = self.run_inference(session, tokens, style, speed)?;

        // Apply gain/amplification if requested
        if gain != 1.0 {
            audio = amplify_audio(&audio, gain);
        }

        Ok(audio)
    }

    /// Save audio as WAV file
    pub fn save_wav(&self, path: &str, audio: &[f32]) -> Result<(), String> {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: SAMPLE_RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = hound::WavWriter::create(path, spec)
            .map_err(|e| format!("Failed to create WAV file: {}", e))?;

        for &sample in audio {
            let sample_i16 = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
            writer.write_sample(sample_i16)
                .map_err(|e| format!("Failed to write sample: {}", e))?;
        }

        writer.finalize()
            .map_err(|e| format!("Failed to finalize WAV: {}", e))?;
        Ok(())
    }

    /// Convert audio to WAV bytes in memory
    pub fn to_wav_bytes(&self, audio: &[f32]) -> Result<Vec<u8>, String> {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: SAMPLE_RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = hound::WavWriter::new(&mut cursor, spec)
                .map_err(|e| format!("Failed to create WAV writer: {}", e))?;

            for &sample in audio {
                let sample_i16 = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
                writer.write_sample(sample_i16)
                    .map_err(|e| format!("Failed to write sample: {}", e))?;
            }

            writer.finalize()
                .map_err(|e| format!("Failed to finalize WAV: {}", e))?;
        }

        Ok(cursor.into_inner())
    }

    /// Save audio as MP3 file (requires 'mp3' feature)
    #[cfg(feature = "mp3")]
    pub fn save_mp3(&self, path: &str, audio: &[f32]) -> Result<(), String> {
        use mp3lame_encoder::{Builder, InterleavedPcm};

        let mut encoder = Builder::new()
            .ok_or("Failed to create MP3 encoder builder")?
            .sample_rate(SAMPLE_RATE)
            .ok_or("Invalid sample rate")?
            .channels(mp3lame_encoder::channels::Mono)
            .ok_or("Failed to set mono channel")?
            .quality(mp3lame_encoder::Quality::Best)
            .ok_or("Failed to set quality")?
            .build()
            .map_err(|e| format!("Failed to build MP3 encoder: {:?}", e))?;

        // Convert to i16
        let samples_i16: Vec<i16> = audio.iter()
            .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
            .collect();

        let pcm = InterleavedPcm(&samples_i16);
        let mut mp3_data = Vec::new();

        let mut output = [0u8; 8192];
        let encoded_size = encoder.encode(pcm, &mut output)
            .map_err(|e| format!("Failed to encode MP3: {:?}", e))?;
        mp3_data.extend_from_slice(&output[..encoded_size]);

        let final_size = encoder.flush(&mut output)
            .map_err(|e| format!("Failed to flush MP3 encoder: {:?}", e))?;
        mp3_data.extend_from_slice(&output[..final_size]);

        std::fs::write(path, mp3_data)
            .map_err(|e| format!("Failed to write MP3 file: {}", e))?;

        Ok(())
    }

    /// Save audio as OPUS file (requires 'opus-format' feature)
    #[cfg(feature = "opus-format")]
    pub fn save_opus(&self, path: &str, audio: &[f32], bitrate: i32) -> Result<(), String> {
        use audiopus::{coder::Encoder as OpusEncoder, Channels, Application, Bitrate, SampleRate};

        // Convert sample rate from 24000 to 48000 (OPUS prefers 48kHz)
        let samples_48k = resample_audio(audio, SAMPLE_RATE, 48000);

        // Convert to i16
        let samples_i16: Vec<i16> = samples_48k.iter()
            .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
            .collect();

        // Create OPUS encoder
        let mut encoder = OpusEncoder::new(
            SampleRate::Hz48000,
            Channels::Mono,
            Application::Audio
        ).map_err(|e| format!("Failed to create OPUS encoder: {:?}", e))?;

        // Set bitrate
        encoder.set_bitrate(Bitrate::BitsPerSecond(bitrate))
            .map_err(|e| format!("Failed to set OPUS bitrate: {:?}", e))?;

        // Encode in chunks
        let frame_size = 960; // 20ms at 48kHz
        let mut opus_data = Vec::new();
        let mut output = vec![0u8; 4000];

        for chunk in samples_i16.chunks(frame_size) {
            if chunk.len() == frame_size {
                let size = encoder.encode(chunk, &mut output)
                    .map_err(|e| format!("Failed to encode OPUS frame: {:?}", e))?;
                opus_data.extend_from_slice(&output[..size]);
            }
        }

        std::fs::write(path, opus_data)
            .map_err(|e| format!("Failed to write OPUS file: {}", e))?;

        Ok(())
    }


    /// Save audio file with automatic format detection based on extension
    pub fn save_audio(&self, path: &str, audio: &[f32]) -> Result<(), String> {
        let extension = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("wav")
            .to_lowercase();

        match extension.as_str() {
            "wav" => self.save_wav(path, audio),

            #[cfg(feature = "mp3")]
            "mp3" => self.save_mp3(path, audio),
            #[cfg(not(feature = "mp3"))]
            "mp3" => Err("MP3 support not enabled. Add 'mp3' feature to Cargo.toml".to_string()),

            #[cfg(feature = "opus-format")]
            "opus" => self.save_opus(path, audio, 24000),
            #[cfg(not(feature = "opus-format"))]
            "opus" => Err("OPUS support not enabled. Add 'opus-format' feature to Cargo.toml".to_string()),

            "flac" => Err("FLAC format not yet supported".to_string()),

            _ => Err(format!("Unsupported audio format: {}", extension)),
        }
    }

    /// Play audio directly through speakers (requires 'playback' feature)
    #[cfg(feature = "playback")]
    pub fn play(&self, audio: &[f32], volume: f32) -> Result<(), String> {
        self.play_with_ducking(audio, volume, false, 0.3)
    }

    /// Play audio with optional ducking (requires 'playback' feature)
    /// Ducking reduces system volume before speaking, then restores it after
    ///
    /// # Arguments
    /// * `audio` - Audio samples to play
    /// * `volume` - Playback volume (0.0 to 1.0)
    /// * `enable_ducking` - Whether to reduce other audio during playback
    /// * `duck_level` - How much to reduce other audio (0.0 = mute, 1.0 = no change)
    #[cfg(feature = "playback")]
    pub fn play_with_ducking(&self, audio: &[f32], volume: f32, enable_ducking: bool, duck_level: f32) -> Result<(), String> {
        // Duck audio if requested (reduce system volume)
        #[cfg(feature = "ducking")]
        if enable_ducking {
            duck_system_audio(duck_level)?;
            // Small delay to let the ducking take effect
            thread::sleep(Duration::from_millis(50));
        }

        // Convert audio to WAV format in memory
        let wav_data = self.to_wav_bytes(audio)?;

        // Setup audio output
        let (_stream, stream_handle) = OutputStream::try_default()
            .map_err(|e| format!("Failed to get audio output: {}", e))?;

        let sink = Sink::try_new(&stream_handle)
            .map_err(|e| format!("Failed to create audio sink: {}", e))?;

        // Set volume (0.0 to 1.0)
        sink.set_volume(volume.clamp(0.0, 1.0));

        // Create decoder from WAV data
        let cursor = Cursor::new(wav_data);
        let decoder = Decoder::new(cursor)
            .map_err(|e| format!("Failed to create audio decoder: {}", e))?;

        // Play the audio
        sink.append(decoder);
        sink.sleep_until_end();

        // Restore audio if we ducked it
        #[cfg(feature = "ducking")]
        if enable_ducking {
            // Small delay before restoring
            thread::sleep(Duration::from_millis(50));
            restore_system_audio(duck_level)?;
        }

        Ok(())
    }

    // Private helper methods

    fn parse_voice_style(&self, voice_str: &str) -> Result<Vec<f32>, String> {
        if self.fallback_mode {
            // Return a dummy style vector for fallback mode
            return Ok(vec![0.0; 256]);
        }

        let mut result = vec![0.0; 256];
        let parts: Vec<&str> = voice_str.split('+').collect();

        for part in parts {
            let (voice_name, weight) = if part.contains('.') {
                let pieces: Vec<&str> = part.split('.').collect();
                if pieces.len() != 2 {
                    return Err(format!("Invalid voice format: {}", part));
                }
                let weight = pieces[1].parse::<f32>()
                    .map_err(|_| format!("Invalid weight: {}", pieces[1]))?;
                (pieces[0], weight / 10.0)
            } else {
                (part, 1.0)
            };

            let voice_style = self.voices.get(voice_name)
                .ok_or_else(|| format!("Voice not found: {}", voice_name))?;

            for (i, val) in voice_style.iter().enumerate() {
                if i < result.len() {
                    result[i] += val * weight;
                }
            }
        }

        Ok(result)
    }

    fn tokenize(&self, text: String) -> Vec<i64> {
        text.chars()
            .map(|c| *self.vocab.get(&c).unwrap_or(&0))
            .collect()
    }

    fn run_inference(
        &self,
        session: &Arc<Mutex<Session>>,
        tokens: Vec<i64>,
        style: Vec<f32>,
        speed: f32
    ) -> Result<Vec<f32>, String> {
        let mut session = session.lock()
            .map_err(|e| format!("Failed to lock session: {}", e))?;

        let token_count = tokens.len();  // Save count before moving

        // Prepare tokens tensor
        let tokens_array = ndarray::Array2::from_shape_vec((1, tokens.len()), tokens)
            .map_err(|e| format!("Failed to create tokens array: {}", e))?;
        let tokens_tensor = Tensor::from_array(tokens_array)
            .map_err(|e| format!("Failed to create tokens tensor: {}", e))?;

        // Prepare style tensor
        let style_array = ndarray::Array2::from_shape_vec((1, style.len()), style)
            .map_err(|e| format!("Failed to create style array: {}", e))?;
        let style_tensor = Tensor::from_array(style_array)
            .map_err(|e| format!("Failed to create style tensor: {}", e))?;

        // Prepare speed tensor
        let speed_array = ndarray::Array1::from_vec(vec![speed]);
        let speed_tensor = Tensor::from_array(speed_array)
            .map_err(|e| format!("Failed to create speed tensor: {}", e))?;

        // Create inputs
        use std::borrow::Cow;
        let inputs = SessionInputs::from(vec![
            (Cow::Borrowed("tokens"), SessionInputValue::Owned(Value::from(tokens_tensor))),
            (Cow::Borrowed("style"), SessionInputValue::Owned(Value::from(style_tensor))),
            (Cow::Borrowed("speed"), SessionInputValue::Owned(Value::from(speed_tensor))),
        ]);

        // Run inference
        let outputs = session.run(inputs)
            .map_err(|e| format!("Failed to run inference: {}", e))?;

        // Extract audio
        let (shape, data) = outputs["audio"]
            .try_extract_tensor::<f32>()
            .map_err(|e| format!("Failed to extract audio tensor: {}", e))?;

        // Debug output shape for longer text
        let data_vec = data.to_vec();
        if token_count > 100 {
            eprintln!("   Output audio shape: {:?}, samples: {}", shape, data_vec.len());
        }

        Ok(data_vec)
    }
}

// Helper functions

// Build proper vocabulary for tokenization (matching original Kokoros)
fn build_vocab() -> HashMap<char, i64> {
    let pad = "$";
    let punctuation = r#";:,.!?¡¿—…"«»"" "#;
    let letters = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let letters_ipa = "ɑɐɒæɓʙβɔɕçɗɖðʤəɘɚɛɜɝɞɟʄɡɠɢʛɦɧħɥʜɨɪʝɭɬɫɮʟɱɯɰŋɳɲɴøɵɸθœɶʘɹɺɾɻʀʁɽʂʃʈʧʉʊʋⱱʌɣɤʍχʎʏʑʐʒʔʡʕʢǀǁǂǃˈˌːˑʼʴʰʱʲʷˠˤ˞↓↑→↗↘'̩'ᵻ";

    let symbols: String = [pad, punctuation, letters, letters_ipa].concat();

    symbols
        .chars()
        .enumerate()
        .map(|(idx, c)| (c, idx as i64))
        .collect()
}

// Load voices from binary file
fn load_voices(path: &str) -> Result<HashMap<String, Vec<f32>>, String> {
    let mut file = File::open(path)
        .map_err(|e| format!("Failed to open voices file: {}", e))?;

    let mut reader = NpzReader::new(&mut file)
        .map_err(|e| format!("Failed to create NPZ reader: {}", e))?;

    let mut voices = HashMap::new();

    for name in reader.names().map_err(|e| format!("Failed to read NPZ names: {:?}", e))? {
        let array: ArrayBase<OwnedRepr<f32>, IxDyn> = reader.by_name(&name)
            .map_err(|e| format!("Failed to read NPZ array {}: {:?}", name, e))?;
        let data: Vec<f32> = array.iter().cloned().collect();

        // Clean up the name (remove .npy extension if present)
        let clean_name = name.trim_end_matches(".npy");
        voices.insert(clean_name.to_string(), data);
    }

    Ok(voices)
}

// Download file from URL
async fn download_file(url: &str, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let response = reqwest::get(url).await?;
    let bytes = response.bytes().await?;

    let mut file = File::create(path)?;
    file.write_all(&bytes)?;

    Ok(())
}

// Convert WAV bytes to f32 samples
fn wav_to_f32(wav_bytes: &[u8]) -> Result<Vec<f32>, String> {
    let cursor = Cursor::new(wav_bytes);
    let mut reader = hound::WavReader::new(cursor)
        .map_err(|e| format!("Failed to read WAV: {}", e))?;

    let samples: Result<Vec<f32>, _> = reader.samples::<i16>()
        .map(|s| s.map(|sample| sample as f32 / 32768.0))
        .collect();

    samples.map_err(|e| format!("Failed to read samples: {}", e))
}

// Simple audio resampling (for OPUS)
#[cfg(feature = "opus-format")]
fn resample_audio(input: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    let ratio = to_rate as f32 / from_rate as f32;
    let new_len = (input.len() as f32 * ratio) as usize;
    let mut output = Vec::with_capacity(new_len);

    for i in 0..new_len {
        let src_idx = i as f32 / ratio;
        let idx_floor = src_idx.floor() as usize;
        let idx_ceil = (idx_floor + 1).min(input.len() - 1);
        let fraction = src_idx - idx_floor as f32;

        let sample = if idx_floor < input.len() {
            input[idx_floor] * (1.0 - fraction) + input[idx_ceil] * fraction
        } else {
            0.0
        };

        output.push(sample);
    }

    output
}

// Split text into chunks for better synthesis
// Kokoro model handles shorter text better without dropping words
fn split_text_for_tts(text: &str, max_chars: usize) -> Vec<String> {
    // First try to split by sentences
    let sentences: Vec<&str> = text.split_terminator(&['.', '!', '?'][..])
        .filter(|s| !s.trim().is_empty())
        .collect();

    let mut chunks = Vec::new();
    let mut current_chunk = String::new();

    for sentence in sentences {
        // Add back the punctuation if it was there
        let full_sentence = if text.contains(&format!("{}.", sentence.trim())) {
            format!("{}.", sentence.trim())
        } else if text.contains(&format!("{}!", sentence.trim())) {
            format!("{}!", sentence.trim())
        } else if text.contains(&format!("{}?", sentence.trim())) {
            format!("{}?", sentence.trim())
        } else {
            sentence.trim().to_string()
        };

        // If this sentence alone is too long, split it by commas or words
        if full_sentence.len() > max_chars {
            // Try splitting by commas first
            let parts: Vec<&str> = full_sentence.split(',').collect();
            if parts.len() > 1 {
                for part in parts {
                    if part.trim().len() > max_chars {
                        // Still too long, split by words
                        chunks.extend(split_by_words(part, max_chars));
                    } else if !part.trim().is_empty() {
                        chunks.push(part.trim().to_string());
                    }
                }
            } else {
                // No commas, split by words
                chunks.extend(split_by_words(&full_sentence, max_chars));
            }
        }
        // If adding this sentence would make chunk too long, save current and start new
        else if !current_chunk.is_empty() && current_chunk.len() + full_sentence.len() + 1 > max_chars {
            chunks.push(current_chunk.trim().to_string());
            current_chunk = full_sentence;
        }
        // Add to current chunk
        else {
            if !current_chunk.is_empty() {
                current_chunk.push(' ');
            }
            current_chunk.push_str(&full_sentence);
        }
    }

    // Don't forget the last chunk
    if !current_chunk.is_empty() {
        chunks.push(current_chunk.trim().to_string());
    }

    // If no chunks were created (text had no sentence endings), split by words
    if chunks.is_empty() && !text.trim().is_empty() {
        chunks = split_by_words(text, max_chars);
    }

    chunks
}

// Split text by words when sentences are too long
fn split_by_words(text: &str, max_chars: usize) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut chunks = Vec::new();
    let mut current = String::new();

    for word in words {
        if current.len() + word.len() + 1 > max_chars && !current.is_empty() {
            chunks.push(current.trim().to_string());
            current = word.to_string();
        } else {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
    }

    if !current.is_empty() {
        chunks.push(current.trim().to_string());
    }

    chunks
}

// Amplify audio - allows some clipping for maximum loudness
fn amplify_audio(audio: &[f32], gain: f32) -> Vec<f32> {
    audio.iter().map(|&sample| {
        let amplified = sample * gain;

        // Simple hard clipping at the limits
        // This allows maximum volume even if it distorts a bit
        amplified.clamp(-1.0, 1.0)
    }).collect()
}

// BabyTts implementation for mem8 integration
impl BabyTts {
    /// Create a new baby TTS for mem8 learning
    pub async fn new() -> Result<Self, String> {
        let engine = TtsEngine::new().await?;
        Ok(Self {
            engine,
            max_words: 5,  // Babies start with short phrases
            voice: "af_sky".to_string(),  // Gentle voice for baby
            speed: 0.9,  // Slightly slower for clarity
            gain: 1.8,   // Louder for clarity
        })
    }

    /// Create with custom settings
    pub async fn with_settings(max_words: usize, voice: &str, speed: f32, gain: f32) -> Result<Self, String> {
        let engine = TtsEngine::new().await?;
        Ok(Self {
            engine,
            max_words,
            voice: voice.to_string(),
            speed,
            gain,
        })
    }

    /// Speak a simple utterance (for mem8 baby learning)
    pub fn speak(&mut self, text: &str) -> Result<Vec<f32>, String> {
        // Limit to max_words for baby speech
        let words: Vec<&str> = text.split_whitespace().collect();
        let limited_text = if words.len() > self.max_words {
            eprintln!("🍼 Baby mode: Limiting to {} words", self.max_words);
            words[..self.max_words].join(" ")
        } else {
            text.to_string()
        };

        // Synthesize with baby settings
        self.engine.synthesize_with_options(&limited_text, Some(&self.voice), self.speed, self.gain)
    }

    /// Get raw audio samples at 24kHz (for mem8 processing)
    pub fn get_audio_params(&self) -> (u32, u16, u16) {
        (SAMPLE_RATE, 1, 16)  // 24kHz, mono, 16-bit
    }

    /// Process incoming audio for learning (placeholder for mem8 integration)
    pub fn learn_from_audio(&mut self, audio: &[f32], text: &str) -> Result<(), String> {
        // This would integrate with mem8's learning system
        // For now, just log the learning attempt
        eprintln!("🧠 Baby learning: '{}' ({} samples)", text, audio.len());
        Ok(())
    }

    /// Babble - generate random baby sounds (for early development stages)
    pub fn babble(&mut self) -> Result<Vec<f32>, String> {
        let baby_sounds = ["ma", "ba", "da", "goo", "ga", "baba", "mama", "dada"];
        // Simple pseudo-random using current time
        let index = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize) % baby_sounds.len();
        let sound = baby_sounds[index];
        self.speak(sound)
    }

    /// Echo mode - repeat what was heard (for learning)
    pub fn echo(&mut self, text: &str) -> Result<Vec<f32>, String> {
        // Simple echo with slightly different intonation
        let echo_speed = self.speed * 1.1;  // Slightly faster for echo
        self.engine.synthesize_with_options(text, Some(&self.voice), echo_speed, self.gain)
    }

    /// Grow vocabulary - increase max words as baby learns
    pub fn grow(&mut self) {
        self.max_words = (self.max_words + 1).min(20);  // Cap at 20 words for kokoro-tiny
        eprintln!("🌱 Baby growing! Can now speak {} words at once", self.max_words);
    }
}

// Audio ducking functions - reduce system volume during TTS playback
#[cfg(feature = "ducking")]
fn duck_system_audio(level: f32) -> Result<(), String> {
    // Calculate how many volume-down presses we need
    // Assuming each press is ~6% volume change on most systems
    let steps = ((1.0 - level) * 16.0) as u32; // 16 steps = ~100% volume range

    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| format!("Failed to initialize Enigo: {:?}", e))?;

    // Press volume down keys to reduce system volume
    for _ in 0..steps {
        enigo.key(Key::VolumeDown, enigo::Direction::Click)
            .map_err(|e| format!("Failed to press volume down: {:?}", e))?;
        thread::sleep(Duration::from_millis(20)); // Small delay between key presses
    }

    Ok(())
}

#[cfg(feature = "ducking")]
fn restore_system_audio(level: f32) -> Result<(), String> {
    // Calculate how many volume-up presses to restore
    let steps = ((1.0 - level) * 16.0) as u32;

    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| format!("Failed to initialize Enigo: {:?}", e))?;

    // Press volume up keys to restore system volume
    for _ in 0..steps {
        enigo.key(Key::VolumeUp, enigo::Direction::Click)
            .map_err(|e| format!("Failed to press volume up: {:?}", e))?;
        thread::sleep(Duration::from_millis(20)); // Small delay between key presses
    }

    Ok(())
}