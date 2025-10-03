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

use espeak_rs::text_to_phonemes;
use ndarray::{ArrayBase, IxDyn, OwnedRepr};
use ndarray_npy::NpzReader;
use ort::{
    session::{builder::GraphOptimizationLevel, Session, SessionInputs, SessionInputValue},
    value::{Tensor, Value},
};

#[cfg(feature = "playback")]
use rodio::{Decoder, OutputStream, Sink, Source};
use std::io::Cursor;

// Constants
const MODEL_URL: &str = "https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0/kokoro-v1.0.onnx";
const VOICES_URL: &str = "https://github.com/thewh1teagle/kokoro-onnx/releases/download/model-files-v1.0/voices-v1.0.bin";
const SAMPLE_RATE: u32 = 24000;
const DEFAULT_VOICE: &str = "af_sky";
const DEFAULT_SPEED: f32 = 1.0;
// Pad token used to mark sequence boundaries (must be present in vocabulary)
const PAD: &str = "$";

// Get cache directory for shared model storage (Hue's suggestion!)
fn get_cache_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    Path::new(&home).join(".cache").join("kokoros")
}

/// Main TTS engine struct
pub struct TtsEngine {
    session: Arc<Mutex<Session>>,
    voices: HashMap<String, Vec<f32>>,
    vocab: HashMap<char, i64>,
}

impl TtsEngine {
    /// Create a new TTS engine, downloading model files if necessary
    /// Uses ~/.cache/kokoros for shared model storage
    pub async fn new() -> Result<Self, String> {
        let cache_dir = get_cache_dir();
        let model_path = cache_dir.join("kokoro-v1.0.onnx");
        let voices_path = cache_dir.join("voices-v1.0.bin");

        Self::with_paths(
            model_path.to_str().unwrap_or("kokoro-v1.0.onnx"),
            voices_path.to_str().unwrap_or("voices-v1.0.bin")
        ).await
    }

    /// Create a new TTS engine with custom model paths
    pub async fn with_paths(model_path: &str, voices_path: &str) -> Result<Self, String> {
        // Ensure cache directory exists
        if let Some(parent) = Path::new(model_path).parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create cache directory: {}", e))?;
        }

        // Download model if needed
        if !Path::new(model_path).exists() {
            download_file(MODEL_URL, model_path).await
                .map_err(|e| format!("Failed to download model: {}", e))?;
        }

        // Download voices if needed
        if !Path::new(voices_path).exists() {
            download_file(VOICES_URL, voices_path).await
                .map_err(|e| format!("Failed to download voices: {}", e))?;
        }

        // Load ONNX model
        let model_bytes = std::fs::read(model_path)
            .map_err(|e| format!("Failed to read model file: {}", e))?;
        let session = Session::builder()
            .map_err(|e| format!("Failed to create session builder: {}", e))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| format!("Failed to set optimization level: {}", e))?
            .with_intra_threads(4)
            .map_err(|e| format!("Failed to set intra threads: {}", e))?
            .commit_from_memory(&model_bytes)
            .map_err(|e| format!("Failed to load model: {}", e))?;

        // Load voices
        let voices = load_voices(voices_path)
            .map_err(|e| format!("Failed to load voices: {}", e))?;

        // Create proper vocabulary mapping (same as original Kokoros)
        let vocab = build_vocab();

        Ok(Self {
            session: Arc::new(Mutex::new(session)),
            voices,
            vocab,
        })
    }

    /// Synthesize speech from text
    pub fn synthesize(&mut self, text: &str, voice: Option<&str>) -> Result<Vec<f32>, String> {
        let voice = voice.unwrap_or(DEFAULT_VOICE);

        // Get voice style
        let style = self.voices.get(voice)
            .ok_or_else(|| format!("Voice '{}' not found", voice))?
            .clone();

        // Convert text to phonemes
        // Parameters: text, language, voice variant (None for default), preserve punctuation, with_stress
        let phonemes = text_to_phonemes(text, "en", None, true, false)
            .map_err(|e| format!("Failed to convert text to phonemes: {:?}", e))?;

    // Join phonemes into a single string and pad at the start/end so short
    // inputs don't lose leading or trailing tokens (e.g. "it's 21:22")
    let inner = phonemes.join("");
    let phonemes_str = format!("{}{}{}", PAD, inner, PAD);

        // Tokenize phonemes using proper vocabulary
        let tokens = self.tokenize(&phonemes_str);

        // Run inference
        let audio = self.infer(tokens, style, DEFAULT_SPEED)?;

        Ok(audio)
    }

    /// Play audio directly to the default audio device with volume control
    #[cfg(feature = "playback")]
    pub fn play(&self, audio: &[f32], volume: f32) -> Result<(), String> {
        // Convert audio to WAV format in memory
        let wav_data = self.to_wav_bytes(audio)?;

        // Setup audio output
        let (_stream, stream_handle) = OutputStream::try_default()
            .map_err(|e| format!("Failed to get audio output: {}", e))?;

        let sink = Sink::try_new(&stream_handle)
            .map_err(|e| format!("Failed to create audio sink: {}", e))?;

        // Decode WAV data from memory
        let cursor = Cursor::new(wav_data);
        let source = Decoder::new(cursor)
            .map_err(|e| format!("Failed to decode audio: {}", e))?;

        // Set volume (0.0 to 1.0)
        sink.set_volume(volume.clamp(0.0, 1.0));

        // Play the audio
        sink.append(source);
        sink.sleep_until_end();

        Ok(())
    }

    /// List available audio devices
    #[cfg(feature = "playback")]
    pub fn list_devices() -> Vec<String> {
        use cpal::traits::{HostTrait, DeviceTrait};

        if let Ok(devices) = cpal::default_host().output_devices() {
            devices.filter_map(|device| device.name().ok()).collect()
        } else {
            vec!["default".to_string()]
        }
    }

    /// Convert audio to WAV bytes (for playback)
    fn to_wav_bytes(&self, audio: &[f32]) -> Result<Vec<u8>, String> {
        let mut buffer = Vec::new();
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: SAMPLE_RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        {
            let mut writer = hound::WavWriter::new(Cursor::new(&mut buffer), spec)
                .map_err(|e| format!("Failed to create WAV writer: {}", e))?;

            for &sample in audio {
                let sample_i16 = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
                writer.write_sample(sample_i16)
                    .map_err(|e| format!("Failed to write sample: {}", e))?;
            }

            writer.finalize()
                .map_err(|e| format!("Failed to finalize WAV: {}", e))?;
        }

        Ok(buffer)
    }

    /// List available voices
    pub fn voices(&self) -> Vec<String> {
        self.voices.keys().cloned().collect()
    }

    /// Save audio to WAV file
    pub fn save_wav(&self, path: &str, audio: &[f32]) -> Result<(), String> {
        // Ensure directory exists
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: SAMPLE_RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = hound::WavWriter::create(path, spec)
            .map_err(|e| format!("Failed to create WAV file: {}", e))?;

        // Convert float samples to i16
        for &sample in audio {
            let sample_i16 = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
            writer.write_sample(sample_i16)
                .map_err(|e| format!("Failed to write sample: {}", e))?;
        }

        writer.finalize()
            .map_err(|e| format!("Failed to finalize WAV: {}", e))?;
        Ok(())
    }

    #[cfg(feature = "mp3")]
    /// Save audio to MP3 file (requires mp3 feature)
    pub fn save_mp3(&self, path: &str, audio: &[f32]) -> Result<(), String> {
        use mp3lame_encoder::{Builder, Encoder, FlushNoGap};

        // Ensure directory exists
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }

        // Convert to i16 samples
        let samples: Vec<i16> = audio.iter()
            .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
            .collect();

        // Setup MP3 encoder
        let mut encoder = Builder::new()
            .map_err(|e| format!("Failed to create MP3 encoder: {:?}", e))?
            .set_num_channels(1)
            .map_err(|e| format!("Failed to set channels: {:?}", e))?
            .set_sample_rate(SAMPLE_RATE)
            .map_err(|e| format!("Failed to set sample rate: {:?}", e))?
            .set_brate(mp3lame_encoder::Bitrate::Kbps128)
            .map_err(|e| format!("Failed to set bitrate: {:?}", e))?
            .set_quality(mp3lame_encoder::Quality::Best)
            .map_err(|e| format!("Failed to set quality: {:?}", e))?
            .build()
            .map_err(|e| format!("Failed to build encoder: {:?}", e))?;

        let mut mp3_data = Vec::new();
        let encoded = encoder.encode(&samples)
            .map_err(|e| format!("Failed to encode: {:?}", e))?;
        mp3_data.extend_from_slice(&encoded);

        let encoded = encoder.flush::<FlushNoGap>()
            .map_err(|e| format!("Failed to flush: {:?}", e))?;
        mp3_data.extend_from_slice(&encoded);

        // Write to file
        let mut file = File::create(path)
            .map_err(|e| format!("Failed to create file: {}", e))?;
        file.write_all(&mp3_data)
            .map_err(|e| format!("Failed to write file: {}", e))?;

        Ok(())
    }

    #[cfg(feature = "opus-format")]
    /// Save audio to OPUS file - great for streaming and low bandwidth!
    pub fn save_opus(&self, path: &str, audio: &[f32], bitrate: i32) -> Result<(), String> {
        use audiopus::{coder::Encoder, Channels, SampleRate, Application};

        // Ensure directory exists
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }

        // Convert to i16 samples
        let samples: Vec<i16> = audio.iter()
            .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
            .collect();

        // Setup OPUS encoder (24kHz mono)
        let mut encoder = Encoder::new(
            SampleRate::Hz24000,
            Channels::Mono,
            Application::Audio
        ).map_err(|e| format!("Failed to create OPUS encoder: {:?}", e))?;

        // Set bitrate (typical: 24000 for speech)
        encoder.set_bitrate(bitrate)
            .map_err(|e| format!("Failed to set bitrate: {:?}", e))?;

        // Encode in chunks (OPUS needs specific frame sizes)
        let frame_size = 480; // 20ms at 24kHz
        let mut opus_data = Vec::new();

        for chunk in samples.chunks(frame_size) {
            let mut encoded = vec![0u8; 4000];
            let len = encoder.encode(chunk, &mut encoded)
                .map_err(|e| format!("Failed to encode OPUS: {:?}", e))?;
            opus_data.extend_from_slice(&encoded[..len]);
        }

        // Write to file
        let mut file = File::create(path)
            .map_err(|e| format!("Failed to create file: {}", e))?;
        file.write_all(&opus_data)
            .map_err(|e| format!("Failed to write file: {}", e))?;

        Ok(())
    }

    #[cfg(feature = "flac-format")]
    /// Save audio to FLAC file - lossless quality for when you're getting FLAC!
    pub fn save_flac(&self, path: &str, audio: &[f32]) -> Result<(), String> {
        use flac::StreamWriter;

        // Ensure directory exists
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }

        // Convert to i32 samples (24-bit audio in 32-bit container)
        let samples: Vec<i32> = audio.iter()
            .map(|&s| (s * 8388607.0).clamp(-8388608.0, 8388607.0) as i32)
            .collect();

        // Create FLAC writer
        let file = File::create(path)
            .map_err(|e| format!("Failed to create file: {}", e))?;

        let mut writer = StreamWriter::new(file, 24)
            .map_err(|e| format!("Failed to create FLAC writer: {:?}", e))?;

        // Write samples
        for sample in samples {
            writer.write_sample(sample)
                .map_err(|e| format!("Failed to write FLAC sample: {:?}", e))?;
        }

        // Finalize
        writer.finalize()
            .map_err(|e| format!("Failed to finalize FLAC: {:?}", e))?;

        Ok(())
    }

    /// Save audio in any supported format based on file extension
    pub fn save_audio(&self, path: &str, audio: &[f32]) -> Result<(), String> {
        let extension = Path::new(path)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();

        match extension.as_str() {
            "wav" => self.save_wav(path, audio),
            #[cfg(feature = "mp3")]
            "mp3" => self.save_mp3(path, audio),
            #[cfg(feature = "opus-format")]
            "opus" | "ogg" => self.save_opus(path, audio, 24000),
            #[cfg(feature = "flac-format")]
            "flac" => self.save_flac(path, audio),
            _ => Err(format!("Unsupported audio format: {}", extension))
        }
    }

    // Internal methods

    fn tokenize(&self, phonemes: &str) -> Vec<Vec<i64>> {
        // Use proper vocabulary-based tokenization like original Kokoros
        let tokens: Vec<i64> = phonemes
            .chars()
            .filter_map(|c| self.vocab.get(&c).copied())
            .collect();

        // Return as a batch of one sequence
        vec![tokens]
    }

    fn infer(&mut self, tokens: Vec<Vec<i64>>, style: Vec<f32>, speed: f32) -> Result<Vec<f32>, String> {
        let mut session = self.session.lock().unwrap();

        // Prepare tokens tensor
        let tokens_shape = [tokens.len(), tokens[0].len()];
        let tokens_flat: Vec<i64> = tokens.into_iter().flatten().collect();
        let tokens_tensor = Tensor::from_array((tokens_shape, tokens_flat))
            .map_err(|e| format!("Failed to create tokens tensor: {}", e))?;

        // Prepare style tensor
        let style_shape = [1, style.len()];
        let style_tensor = Tensor::from_array((style_shape, style))
            .map_err(|e| format!("Failed to create style tensor: {}", e))?;

        // Prepare speed tensor
        let speed_tensor = Tensor::from_array(([1], vec![speed]))
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
        let (_shape, data) = outputs["audio"]
            .try_extract_tensor::<f32>()
            .map_err(|e| format!("Failed to extract audio tensor: {}", e))?;

        Ok(data.to_vec())
    }
}

// Helper functions

// Build proper vocabulary for tokenization (matching original Kokoros)
fn build_vocab() -> HashMap<char, i64> {
    let punctuation = r#";:,.!?¡¿—…"«»" "#;
    let letters = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let letters_ipa = "ɑɐɒæɓʙβɔɕçɗɖðʤəɘɚɛɜɝɞɟʄɡɠɢʛɦɧħɥʜɨɪʝɭɬɫɮʟɱɯɰŋɳɲɴøɵɸθœɶʘɹɺɾɻʀʁɽʂʃʈʧʉʊʋⱱʌɣɤʍχʎʏʑʐʒʔʡʕʢǀǁǂǃˈˌːˑʼʴʰʱʲʷˠˤ˞↓↑→↗↘'̩'ᵻ";

    // Ensure PAD is included first so it gets index 0 (matching original vocab)
    let symbols: String = [PAD, punctuation, letters, letters_ipa].concat();

    symbols
        .chars()
        .enumerate()
        .map(|(idx, c)| (c, idx as i64))
        .collect()
}

fn load_voices(path: &str) -> Result<HashMap<String, Vec<f32>>, String> {
    let mut npz = NpzReader::new(File::open(path).map_err(|e| format!("Failed to open voices file: {}", e))?)
        .map_err(|e| format!("Failed to read NPZ: {:?}", e))?;
    let mut voices = HashMap::new();

    for name in npz.names().map_err(|e| format!("Failed to get NPZ names: {:?}", e))? {
        // Read the array directly to avoid type issues
        let arr: ArrayBase<OwnedRepr<f32>, IxDyn> = npz.by_name(&name)
            .map_err(|e| format!("Failed to read voice {}: {:?}", name, e))?;

        // The voice data is shape (510, 1, 256) - we take the first style variation (index 0)
        // Each voice has 510 style variations, we'll use the first one by default
        let shape = arr.shape();
        if shape.len() == 3 && shape[1] == 1 && shape[2] == 256 {
            // Extract just the first style (256 floats)
            let start = 0;
            let end = 256;
            let data = arr.as_slice()
                .ok_or_else(|| format!("Failed to get slice for voice {}", name))?[start..end]
                .to_vec();
            voices.insert(name.trim_end_matches(".npy").to_string(), data);
        }
    }

    Ok(voices)
}

async fn download_file(url: &str, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Ensure directory exists
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }

    println!("Downloading {} to {}...", url, path);

    let response = reqwest::get(url).await?;
    let bytes = response.bytes().await?;

    let mut file = File::create(path)?;
    file.write_all(&bytes)?;

    println!("Downloaded successfully!");
    Ok(())
}

// Simple builder pattern for customization
pub struct TtsBuilder {
    model_path: String,
    voices_path: String,
}

impl Default for TtsBuilder {
    fn default() -> Self {
        let cache_dir = get_cache_dir();
        Self {
            model_path: cache_dir.join("kokoro-v1.0.onnx").to_str().unwrap_or("kokoro-v1.0.onnx").to_string(),
            voices_path: cache_dir.join("voices-v1.0.bin").to_str().unwrap_or("voices-v1.0.bin").to_string(),
        }
    }
}

impl TtsBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn model_path(mut self, path: &str) -> Self {
        self.model_path = path.to_string();
        self
    }

    pub fn voices_path(mut self, path: &str) -> Self {
        self.voices_path = path.to_string();
        self
    }

    pub async fn build(self) -> Result<TtsEngine, String> {
        TtsEngine::with_paths(&self.model_path, &self.voices_path).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_engine_creation() {
        // This would need the model files to be present
        // let engine = TtsEngine::new().await;
        // assert!(engine.is_ok());
    }

    #[test]
    fn test_builder_pattern() {
        let _builder = TtsBuilder::new()
            .model_path("custom_model.onnx")
            .voices_path("custom_voices.bin");
    }

    #[test]
    fn test_vocab_contains_pad() {
        let vocab = build_vocab();
        // PAD should be present as the first symbol ('$')
        assert!(vocab.contains_key(&('$')),
            "Vocabulary must contain the pad token '$'");
    }
}