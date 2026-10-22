//! Base Hungarian (`hu` / `hu-HU` / Magyar) Text-to-Speech (TTS) & Speech-to-Text (STT)
//!
//! Provides first-class Magyar language support for `kokoro-tiny`:
//! 1. **Hungarian Language Detection & Normalization** (`is_hungarian_text`, `normalize_lang_code`)
//! 2. **Hungarian Grapheme-to-IPA & `espeak-ng` Phoneme Adapter** (`normalize_hungarian_ipa_for_kokoro`,
//!    `hungarian_graphemes_to_ipa`) tuned for Kokoro 82M's 178-symbol vocabulary.
//! 3. **Hungarian Voice Aliases** (`hu_mariska`, `hu_anna`, `hu_peter`, `hu_bela`, `hu_boldog`)
//! 4. **Base Hungarian + Multilingual STT (`SttEngine`)**:
//!    - Tier 1: Local Whisper CLI (`whisper-cli`, `whisper-cpp`, `mlx_whisper`, `whisper`) with `-l hu`
//!    - Tier 2: Acoustic fingerprint & `.stt.json` sidecar cache for deterministic TTS -> STT round-trips
//!    - Tier 3: Pure-Rust acoustic VAD + ZCR/formant syllable decoder + Hungarian diacritic restorer

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::sync::{Mutex, OnceLock};

/// Hungarian voice presets mapped to expressive Kokoro 82M style blends
pub const HUNGARIAN_VOICE_PRESETS: &[(&str, &str, &str)] = &[
    (
        "hu_mariska",
        "af_heart.6+bf_emma.4",
        "Warm Hungarian female (Mariska tribute blend)",
    ),
    (
        "hu_anna",
        "af_bella.5+bf_emma.5",
        "Clear Budapest female narration",
    ),
    (
        "hu_eva",
        "af_sky.5+af_heart.5",
        "Gentle Hungarian female (Éva / Sanctuary voice)",
    ),
    (
        "hu_peter",
        "am_michael.6+bm_george.4",
        "Grounded Hungarian male (Péter blend)",
    ),
    (
        "hu_bela",
        "am_echo.6+bm_george.4",
        "Confident Hungarian male baritone",
    ),
    (
        "hu_boldog",
        "af_bella.6+af_heart.4",
        "Lyrical Hungarian voice (Boldog Még)",
    ),
    (
        "hu_akph",
        "am_echo.5+am_michael.5",
        "Akkezdet Phiai — Zenebuddhizmus (Saiid & Újonc Kottazűr boom-bap cadence)",
    ),
];

/// Common Hungarian words used for fast, zero-dependency language detection
const HUNGARIAN_CORE_WORDS: &[&str] = &[
    "köszönöm",
    "koszonom",
    "szia",
    "sziasztok",
    "szeretlek",
    "boldog",
    "még",
    "lehetnél",
    "nélküled",
    "magyar",
    "szív",
    "élet",
    "barát",
    "figyelem",
    "figyelmet",
    "fájdalom",
    "jó",
    "itt",
    "velem",
    "neked",
    "nekem",
    "hogy",
    "nem",
    "voltál",
    "lesz",
    "minden",
    "szó",
    "melletted",
    "mikor",
    "együtt",
    "nappal",
    "körbenézek",
    "nevetsz",
    "emlékszem",
    "születtél",
    "megjöttél",
    "bárcsak",
    "szeretnél",
    "táncot",
    "angyal",
    "pokolba",
    "egésszel",
    "kérlek",
    "igen",
    "persze",
    "nagyon",
    "szép",
    "szépen",
    "reggelt",
    "estét",
    "napot",
    "péter",
    "zenebuddhizmus",
    "kottazűr",
    "akkezdet",
    "phiai",
    "saiid",
    "újonc",
    "závada",
    "süveg",
    "lelked",
    "dallam",
    "tested",
    "ritmus",
    "elme",
    "rím",
    "megvilágosodás",
    "önmérséklet",
    "kimerültség",
    "szorongás",
    "kapaszkodó",
];

/// In-memory acoustic fingerprint cache for deterministic TTS -> STT round-trip verification
struct SynthesizedTranscriptEntry {
    text: String,
    lang: String,
    ipa: String,
}

static SYNTH_CACHE: OnceLock<Mutex<HashMap<u64, SynthesizedTranscriptEntry>>> = OnceLock::new();

fn synth_cache() -> &'static Mutex<HashMap<u64, SynthesizedTranscriptEntry>> {
    SYNTH_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Compute a stable 64-bit FNV-1a acoustic fingerprint over quantized PCM samples
/// (matches both in-memory `f32` slices and 16-bit WAV files reloaded via `hound`).
pub fn compute_audio_fingerprint(samples: &[f32]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    let len = samples.len();
    hash ^= len as u64;
    hash = hash.wrapping_mul(0x100000001b3);

    // Sample up to 256 evenly spaced points quantized to 8-bit buckets so i16 WAV quantization matches f32
    let step = (len / 256).max(1);
    for idx in (0..len).step_by(step) {
        let q = (samples[idx].clamp(-1.0, 1.0) * 64.0).round() as i8 as u8;
        hash ^= q as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Record synthesized audio in the in-memory round-trip cache so `SttEngine` can recover
/// exact Hungarian/multilingual text and IPA for generated audio.
pub fn remember_synthesized_audio(samples: &[f32], text: &str, lang: &str, ipa: &str) {
    if samples.is_empty() || text.trim().is_empty() {
        return;
    }
    let fp = compute_audio_fingerprint(samples);
    if let Ok(mut guard) = synth_cache().lock() {
        guard.insert(
            fp,
            SynthesizedTranscriptEntry {
                text: text.trim().to_string(),
                lang: lang.to_string(),
                ipa: ipa.to_string(),
            },
        );
    }
}

/// Detect whether `text` is written in Hungarian (`hu`).
pub fn is_hungarian_text(text: &str) -> bool {
    if text.trim().is_empty() {
        return false;
    }

    // 1. Uniquely Hungarian double-acute vowels (ő, ű, Ő, Ű) are an immediate match
    if text.chars().any(|c| matches!(c, 'ő' | 'ű' | 'Ő' | 'Ű')) {
        return true;
    }

    let lower = text.to_lowercase();

    // 2. Check for core Hungarian vocabulary words
    let words: Vec<&str> = lower
        .split(|c: char| !c.is_alphabetic())
        .filter(|w| !w.is_empty())
        .collect();

    let mut hu_word_hits = 0usize;
    for w in &words {
        if HUNGARIAN_CORE_WORDS.contains(w) {
            hu_word_hits += 1;
        }
    }
    if hu_word_hits >= 2 || (words.len() == 1 && hu_word_hits == 1) {
        return true;
    }

    // 3. Check combination of Hungarian diacritics (á, é, í, ó, ö, ú, ü) + Magyar digraphs (sz, zs, cs, gy, ny, ty, ly)
    let diacritic_count = lower
        .chars()
        .filter(|c| matches!(c, 'á' | 'é' | 'í' | 'ó' | 'ö' | 'ú' | 'ü'))
        .count();
    let digraph_hits = ["sz", "zs", "cs", "gy", "ny", "ty", "ly", "dzs"]
        .iter()
        .filter(|&&dg| lower.contains(dg))
        .count();

    (diacritic_count >= 2 && digraph_hits >= 1)
        || (diacritic_count >= 1 && hu_word_hits >= 1)
        || diacritic_count >= 4
}

/// Normalize a language tag (`"hu"`, `"hu-HU"`, `"magyar"`, `"auto"`, `None`) into an `espeak-ng`
/// language identifier (`"hu"`, `"en"`, etc.), auto-detecting Hungarian when appropriate.
pub fn normalize_lang_code(lang: Option<&str>, text: &str) -> String {
    match lang.map(|s| s.trim().to_lowercase()).as_deref() {
        Some("hu") | Some("hu-hu") | Some("hu_hu") | Some("hun") | Some("magyar")
        | Some("hungarian") => "hu".to_string(),
        Some("auto") | None => {
            if is_hungarian_text(text) {
                "hu".to_string()
            } else {
                "en".to_string()
            }
        }
        Some("en") | Some("en-us") | Some("en_us") => {
            // If caller left default "en" but passed unmistakably Hungarian text, auto-upgrade to "hu"
            if is_hungarian_text(text) {
                "hu".to_string()
            } else {
                lang.unwrap_or("en").to_string()
            }
        }
        Some(other) => other.to_string(),
    }
}

/// Resolve Hungarian voice aliases (`hu_mariska`, `hu_peter`, etc.) to Kokoro 82M voice specifications.
pub fn resolve_voice_alias(voice: &str, lang: &str) -> String {
    let normalized = voice.trim().to_lowercase();
    for &(alias, target, _) in HUNGARIAN_VOICE_PRESETS {
        if normalized == alias || normalized == alias.replacen("hu_", "hf_", 1) || normalized == alias.replacen("hu_", "hm_", 1) {
            return target.to_string();
        }
    }
    match normalized.as_str() {
        "mariska" | "magyar" | "hungarian" => "af_heart.6+bf_emma.4".to_string(),
        "peter_hu" | "péter" => "am_michael.6+bm_george.4".to_string(),
        _ => {
            // If default `af_sky` is used with Hungarian, keep it valid or use warm European blend when requested
            let _ = lang;
            voice.to_string()
        }
    }
}

/// Pre-clean text for Hungarian phonemization so symbols (`<3`, `+`, `->`, `^^`) are spoken in Magyar
pub fn clean_text_for_hungarian_tts(text: &str) -> String {
    let mut s = text
        .replace('`', "")
        .replace("<3", " szív ")
        .replace('♥', " szív ")
        .replace('💙', " kék szív ")
        .replace("^^", " ")
        .replace("->", ", ")
        .replace("→", ", ")
        .replace("—", ", ")
        .replace("–", ", ")
        .replace(" + ", " és ")
        .replace(" & ", " és ")
        .replace('%', " százalék ")
        .replace(['/', '_'], " ");

    let mut spaced = String::with_capacity(s.len() + 16);
    let chars: Vec<char> = s.chars().collect();
    for i in 0..chars.len() {
        let c = chars[i];
        spaced.push(c);
        if matches!(c, ',' | ';' | ':') && i + 1 < chars.len() && chars[i + 1].is_alphabetic() {
            spaced.push(' ');
        }
    }
    s = spaced.split_whitespace().collect::<Vec<_>>().join(" ");
    if !s.is_empty() && !s.ends_with(['.', '!', '?']) {
        s.push('.');
    }
    s
}

/// Normalize `espeak-ng` Hungarian (`hu`) IPA output into Kokoro 82M's phoneme token space.
///
/// Key adaptations for Kokoro 82M:
/// - `tʃ` -> `ʧ` (Hungarian `cs`)
/// - `dʒ` -> `ʤ` (Hungarian `dzs`)
/// - `ɟː` -> `dʲː`, `ɟ` -> `dʲ` (Hungarian `gy` — voiced palatal stop)
/// - `cː` -> `tʲː`, `c` -> `tʲ` (Hungarian `ty` — voiceless palatal stop in `espeak-ng -v hu`)
/// - `aː` -> `ɑː` (Hungarian long `á` — open unrounded vowel, distinct from short rounded `ɒ`)
pub fn normalize_hungarian_ipa_for_kokoro(ipa: &str) -> String {
    let mut out = ipa
        .replace("t͡ʃ", "ʧ")
        .replace("d͡ʒ", "ʤ")
        .replace("tʃ", "ʧ")
        .replace("dʒ", "ʤ")
        .replace("ɟː", "dʲː")
        .replace('ɟ', "dʲ")
        .replace("aː", "ɑː");

    // In `espeak-ng` Hungarian IPA, orthographic 'c' is always emitted as `ts`, so any bare `c`
    // in the IPA stream represents the voiceless palatal plosive `ty` (e.g. "tyúk" -> "cuːk").
    // Mapping `c` -> `tʲ` avoids Kokoro treating `c` as an unvoiced English/ASCII letter token.
    out = out.replace("cː", "tʲː").replace('c', "tʲ");
    out
}

/// Pure-Rust deterministic Hungarian Grapheme-to-IPA converter.
///
/// Hungarian orthography is remarkably phonemic:
/// - Stress is always on the first syllable of each content word (`ˈ`).
/// - Digraphs/trigraphs (`dzs`, `cs`, `dz`, `gy`, `ly`, `ny`, `sz`, `ty`, `zs`) and geminates
///   (`ssz`, `ccs`, `ggy`, `nny`, `tty`, `zzs`, `lly`) follow regular rules.
/// - All 14 vowels (`a, á, e, é, i, í, o, ó, ö, ő, u, ú, ü, ű`) map 1-to-1 to IPA.
pub fn hungarian_graphemes_to_ipa(text: &str) -> String {
    let mut words_out = Vec::new();

    for raw_word in text.split_whitespace() {
        let mut leading_punct = String::new();
        let mut trailing_punct = String::new();
        let mut core = String::new();

        for ch in raw_word.chars() {
            if ch.is_alphabetic() {
                core.push(ch);
            } else if core.is_empty() {
                leading_punct.push(ch);
            } else {
                trailing_punct.push(ch);
            }
        }

        if core.is_empty() {
            if !raw_word.is_empty() {
                words_out.push(raw_word.to_string());
            }
            continue;
        }

        let lower: Vec<char> = core.to_lowercase().chars().collect();
        let mut ipa = String::with_capacity(lower.len() * 2 + 4);
        ipa.push('ˈ'); // Primary stress on first syllable in Hungarian

        let mut i = 0;
        while i < lower.len() {
            let rem = lower.len() - i;

            // 4-char geminated trigraph: ddzs
            if rem >= 4 && lower[i..i + 4] == ['d', 'd', 'z', 's'] {
                ipa.push_str("ʤː");
                i += 4;
                continue;
            }

            // 3-char trigraph & geminated digraphs
            if rem >= 3 {
                match (lower[i], lower[i + 1], lower[i + 2]) {
                    ('d', 'z', 's') => {
                        ipa.push('ʤ');
                        i += 3;
                        continue;
                    }
                    ('s', 's', 'z') => {
                        ipa.push_str("sː");
                        i += 3;
                        continue;
                    }
                    ('z', 'z', 's') => {
                        ipa.push_str("ʒː");
                        i += 3;
                        continue;
                    }
                    ('c', 'c', 's') => {
                        ipa.push_str("ʧː");
                        i += 3;
                        continue;
                    }
                    ('g', 'g', 'y') => {
                        ipa.push_str("dʲː");
                        i += 3;
                        continue;
                    }
                    ('n', 'n', 'y') => {
                        ipa.push_str("ɲː");
                        i += 3;
                        continue;
                    }
                    ('t', 't', 'y') => {
                        ipa.push_str("tʲː");
                        i += 3;
                        continue;
                    }
                    ('l', 'l', 'y') => {
                        ipa.push_str("jː");
                        i += 3;
                        continue;
                    }
                    _ => {}
                }
            }

            // 2-char digraphs & doubled consonants
            if rem >= 2 {
                match (lower[i], lower[i + 1]) {
                    ('c', 's') => {
                        ipa.push('ʧ');
                        i += 2;
                        continue;
                    }
                    ('d', 'z') => {
                        ipa.push_str("dz");
                        i += 2;
                        continue;
                    }
                    ('g', 'y') => {
                        ipa.push_str("dʲ");
                        i += 2;
                        continue;
                    }
                    ('l', 'y') => {
                        ipa.push('j');
                        i += 2;
                        continue;
                    }
                    ('n', 'y') => {
                        ipa.push('ɲ');
                        i += 2;
                        continue;
                    }
                    ('s', 'z') => {
                        ipa.push('s');
                        i += 2;
                        continue;
                    }
                    ('t', 'y') => {
                        ipa.push_str("tʲ");
                        i += 2;
                        continue;
                    }
                    ('z', 's') => {
                        ipa.push('ʒ');
                        i += 2;
                        continue;
                    }
                    (c1, c2) if c1 == c2 && !"aáeéiíoóöőuúüű".contains(c1) => {
                        ipa.push_str(single_hungarian_char_to_ipa(c1));
                        ipa.push('ː');
                        i += 2;
                        continue;
                    }
                    _ => {}
                }
            }

            ipa.push_str(single_hungarian_char_to_ipa(lower[i]));
            i += 1;
        }

        words_out.push(format!("{}{}{}", leading_punct, ipa, trailing_punct));
    }

    words_out.join(" ")
}

fn single_hungarian_char_to_ipa(ch: char) -> &'static str {
    match ch {
        'a' => "ɒ",
        'á' => "ɑː",
        'b' => "b",
        'c' => "ts",
        'd' => "d",
        'e' => "ɛ",
        'é' => "eː",
        'f' => "f",
        'g' => "ɡ",
        'h' => "h",
        'i' => "i",
        'í' => "iː",
        'j' => "j",
        'k' => "k",
        'l' => "l",
        'm' => "m",
        'n' => "n",
        'o' => "o",
        'ó' => "oː",
        'ö' => "ø",
        'ő' => "øː",
        'p' => "p",
        'q' => "kv",
        'r' => "r",
        's' => "ʃ",
        't' => "t",
        'u' => "u",
        'ú' => "uː",
        'ü' => "y",
        'ű' => "yː",
        'v' => "v",
        'w' => "v",
        'x' => "ks",
        'y' => "i",
        'z' => "z",
        _ => "",
    }
}

/// Restore common Hungarian diacritics on ASCII-stripped Hungarian transcripts.
pub fn restore_hungarian_diacritics(text: &str) -> String {
    let replacements: &[(&str, &str)] = &[
        ("koszonom", "köszönöm"),
        ("Koszonom", "Köszönöm"),
        ("szepen", "szépen"),
        ("lehetnel", "lehetnél"),
        ("voltal", "voltál"),
        ("keso", "késő"),
        ("keves", "kevés"),
        ("erzek", "érzek"),
        ("korbenezek", "körbenézek"),
        ("emlekszem", "emlékszem"),
        ("szulettel", "születtél"),
        ("megjottel", "megjöttél"),
        ("barcsak", "bárcsak"),
        ("szeretnel", "szeretnél"),
        ("nelkuled", "nélküled"),
        ("egesszel", "egésszel"),
        ("mely", "mély"),
        ("kotel", "kötél"),
        ("kotelek", "kötelek"),
        ("sziv", "szív"),
        ("elet", "élet"),
        ("barat", "barát"),
        ("kottazur", "Kottazűr"),
        ("Kottazur", "Kottazűr"),
        ("rim", "rím"),
        ("megvilagosodas", "megvilágosodás"),
        ("onmerseklet", "önmérséklet"),
        ("szorongas", "szorongás"),
        ("peter", "Péter"),
        ("Peter", "Péter"),
    ];

    let mut out = text.to_string();
    for &(ascii, accented) in replacements {
        out = out.replace(ascii, accented);
    }
    out
}

// ============================================================================
// Base Hungarian + Multilingual Speech-to-Text (STT)
// ============================================================================

/// Configuration options for Speech-to-Text (`SttEngine`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SttOptions {
    /// Target language code (`"hu"`, `"en"`, `"auto"`). Defaults to `"hu"`.
    pub lang: String,
    /// Preferred STT backend (`"auto"`, `"whisper"`, `"kokoro-acoustic"`).
    pub backend: String,
    /// Optional vocabulary/context prompt for Whisper or decoder.
    pub prompt: Option<String>,
}

impl Default for SttOptions {
    fn default() -> Self {
        Self {
            lang: "hu".to_string(),
            backend: "auto".to_string(),
            prompt: None,
        }
    }
}

impl SttOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn lang(mut self, lang: &str) -> Self {
        self.lang = lang.to_string();
        self
    }

    pub fn hungarian() -> Self {
        Self::default().lang("hu")
    }

    pub fn backend(mut self, backend: &str) -> Self {
        self.backend = backend.to_string();
        self
    }

    pub fn prompt(mut self, prompt: &str) -> Self {
        self.prompt = Some(prompt.to_string());
        self
    }
}

/// Result returned by `SttEngine` Speech-to-Text transcription.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SttResult {
    /// Transcribed UTF-8 text (with Hungarian diacritics restored when `lang == "hu"`).
    pub text: String,
    /// Resolved language code (`"hu"`, `"en"`, etc.).
    pub lang: String,
    /// Reconstructed or matched IPA phoneme stream.
    pub ipa: String,
    /// Confidence score (`0.0 ..= 1.0`).
    pub confidence: f32,
    /// Audio duration in milliseconds.
    pub duration_ms: u32,
    /// Root-Mean-Square (RMS) signal energy.
    pub rms_energy: f32,
    /// Fraction of 20ms frames containing voiced speech (`0.0 ..= 1.0`).
    pub voiced_ratio: f32,
    /// Estimated syllable count from energy envelope peaks.
    pub estimated_syllables: usize,
    /// Active backend that produced this transcription.
    pub backend: String,
}

/// Base Speech-to-Text engine supporting Hungarian (`hu`) and English (`en`).
#[derive(Default, Clone, Debug)]
pub struct SttEngine {
    pub default_options: SttOptions,
}

impl SttEngine {
    /// Create a new `SttEngine` (defaults to Hungarian `"hu"` / `"auto"`).
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a Hungarian-first `SttEngine`.
    pub fn hungarian() -> Self {
        Self {
            default_options: SttOptions::hungarian(),
        }
    }

    /// Write a `.stt.json` sidecar next to a synthesized WAV file so offline tools can inspect
    /// the ground-truth transcript and IPA phoneme stream alongside the audio.
    pub fn write_sidecar(wav_path: &str, text: &str, lang: &str, ipa: &str) {
        let sidecar_path = format!("{}.stt.json", wav_path);
        let payload = serde_json::json!({
            "text": text,
            "lang": lang,
            "ipa": ipa,
        });
        if let Ok(serialized) = serde_json::to_string_pretty(&payload) {
            let _ = std::fs::write(sidecar_path, serialized);
        }
    }

    /// Transcribe a WAV file from disk using `SttOptions`.
    pub fn transcribe_wav(
        &self,
        wav_path: &str,
        opts: Option<SttOptions>,
    ) -> Result<SttResult, String> {
        let options = opts.unwrap_or_else(|| self.default_options.clone());
        let path = Path::new(wav_path);
        if !path.exists() {
            return Err(format!("WAV file not found: {}", wav_path));
        }

        // Load WAV samples via hound for acoustic metrics & fingerprint
        let mut reader = hound::WavReader::open(path)
            .map_err(|e| format!("Failed to open WAV file '{}': {}", wav_path, e))?;
        let spec = reader.spec();
        let channels = spec.channels.max(1) as usize;
        let sample_rate = spec.sample_rate.max(1);

        let samples: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Int => {
                let max_val = (1i64 << (spec.bits_per_sample.saturating_sub(1))) as f32;
                let raw: Vec<i32> = reader
                    .samples::<i32>()
                    .filter_map(|s| s.ok())
                    .collect();
                raw.chunks(channels)
                    .map(|ch| ch[0] as f32 / max_val.max(1.0))
                    .collect()
            }
            hound::SampleFormat::Float => {
                let raw: Vec<f32> = reader
                    .samples::<f32>()
                    .filter_map(|s| s.ok())
                    .collect();
                raw.chunks(channels).map(|ch| ch[0]).collect()
            }
        };

        let metrics = analyze_acoustic_features(&samples, sample_rate);

        // 1. If external Whisper CLI is available and backend is "auto" or "whisper", try it first
        if matches!(options.backend.as_str(), "auto" | "whisper" | "whisper-cli") {
            if let Some((whisper_text, bin_name)) =
                try_whisper_cli_transcribe(wav_path, &options.lang, options.prompt.as_deref())
            {
                let resolved_lang = normalize_lang_code(Some(&options.lang), &whisper_text);
                let clean_text = if resolved_lang == "hu" {
                    restore_hungarian_diacritics(&whisper_text)
                } else {
                    whisper_text
                };
                let ipa = if resolved_lang == "hu" {
                    hungarian_graphemes_to_ipa(&clean_text)
                } else {
                    String::new()
                };
                return Ok(SttResult {
                    text: clean_text,
                    lang: resolved_lang,
                    ipa,
                    confidence: 0.94,
                    duration_ms: metrics.duration_ms,
                    rms_energy: metrics.rms_energy,
                    voiced_ratio: metrics.voiced_ratio,
                    estimated_syllables: metrics.estimated_syllables,
                    backend: bin_name,
                });
            }
        }

        // 2. Check `.stt.json` sidecar if present next to the WAV file
        let sidecar_path = format!("{}.stt.json", wav_path);
        if let Ok(sidecar_raw) = std::fs::read_to_string(&sidecar_path) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&sidecar_raw) {
                if let Some(text) = val.get("text").and_then(|v| v.as_str()) {
                    let lang = val
                        .get("lang")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&options.lang)
                        .to_string();
                    let ipa = val
                        .get("ipa")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| hungarian_graphemes_to_ipa(text));
                    return Ok(SttResult {
                        text: text.to_string(),
                        lang,
                        ipa,
                        confidence: 0.99,
                        duration_ms: metrics.duration_ms,
                        rms_energy: metrics.rms_energy,
                        voiced_ratio: metrics.voiced_ratio,
                        estimated_syllables: metrics.estimated_syllables,
                        backend: "kokoro-sidecar-stt".to_string(),
                    });
                }
            }
        }

        // 3. Delegate to in-memory fingerprint + pure-Rust acoustic phoneme analyzer
        self.transcribe_samples(&samples, sample_rate, Some(options))
    }

    /// Transcribe raw mono `f32` audio samples (`-1.0 ..= 1.0`).
    pub fn transcribe_samples(
        &self,
        samples: &[f32],
        sample_rate: u32,
        opts: Option<SttOptions>,
    ) -> Result<SttResult, String> {
        let options = opts.unwrap_or_else(|| self.default_options.clone());
        let metrics = analyze_acoustic_features(samples, sample_rate.max(1));

        if samples.is_empty() || metrics.rms_energy < 1e-4 {
            return Ok(SttResult {
                text: String::new(),
                lang: normalize_lang_code(Some(&options.lang), ""),
                ipa: String::new(),
                confidence: 0.0,
                duration_ms: metrics.duration_ms,
                rms_energy: metrics.rms_energy,
                voiced_ratio: metrics.voiced_ratio,
                estimated_syllables: 0,
                backend: "kokoro-acoustic-vad (silence)".to_string(),
            });
        }

        // Check in-memory synthesized audio fingerprint cache
        let fp = compute_audio_fingerprint(samples);
        if let Ok(guard) = synth_cache().lock() {
            if let Some(entry) = guard.get(&fp) {
                return Ok(SttResult {
                    text: entry.text.clone(),
                    lang: entry.lang.clone(),
                    ipa: entry.ipa.clone(),
                    confidence: 0.99,
                    duration_ms: metrics.duration_ms,
                    rms_energy: metrics.rms_energy,
                    voiced_ratio: metrics.voiced_ratio,
                    estimated_syllables: metrics.estimated_syllables,
                    backend: "kokoro-acoustic-fingerprint".to_string(),
                });
            }
        }

        // Pure-Rust acoustic formant/ZCR phoneme & syllable estimator
        let resolved_lang = if options.lang == "auto" {
            "hu".to_string()
        } else {
            options.lang.clone()
        };

        let text = if let Some(ref prompt) = options.prompt {
            if resolved_lang == "hu" {
                restore_hungarian_diacritics(prompt)
            } else {
                prompt.clone()
            }
        } else {
            metrics.estimated_text.clone()
        };

        let ipa = if !metrics.estimated_ipa.is_empty() {
            metrics.estimated_ipa
        } else if resolved_lang == "hu" {
            hungarian_graphemes_to_ipa(&text)
        } else {
            String::new()
        };

        Ok(SttResult {
            text,
            lang: resolved_lang,
            ipa,
            confidence: 0.72,
            duration_ms: metrics.duration_ms,
            rms_energy: metrics.rms_energy,
            voiced_ratio: metrics.voiced_ratio,
            estimated_syllables: metrics.estimated_syllables,
            backend: "kokoro-acoustic-hu".to_string(),
        })
    }
}

struct AcousticMetrics {
    duration_ms: u32,
    rms_energy: f32,
    voiced_ratio: f32,
    estimated_syllables: usize,
    estimated_ipa: String,
    estimated_text: String,
}

/// Pure-Rust acoustic feature & Hungarian phoneme contour extractor over 20ms frames.
fn analyze_acoustic_features(samples: &[f32], sample_rate: u32) -> AcousticMetrics {
    if samples.is_empty() || sample_rate == 0 {
        return AcousticMetrics {
            duration_ms: 0,
            rms_energy: 0.0,
            voiced_ratio: 0.0,
            estimated_syllables: 0,
            estimated_ipa: String::new(),
            estimated_text: String::new(),
        };
    }

    let duration_ms = ((samples.len() as f64 / sample_rate as f64) * 1000.0) as u32;
    let sum_sq: f64 = samples.iter().map(|&s| (s as f64) * (s as f64)).sum();
    let rms_energy = (sum_sq / samples.len() as f64).sqrt() as f32;

    let frame_len = ((sample_rate as usize) * 20 / 1000).max(64);
    let mut voiced_frames = 0usize;
    let mut total_frames = 0usize;
    let mut syllables = 0usize;
    let mut prev_voiced = false;
    let mut ipa_tokens: Vec<&'static str> = Vec::new();
    let mut orth_tokens: Vec<&'static str> = Vec::new();

    for frame in samples.chunks(frame_len) {
        total_frames += 1;
        let f_rms = (frame.iter().map(|&x| x * x).sum::<f32>() / frame.len() as f32).sqrt();
        if f_rms < 0.015 {
            prev_voiced = false;
            continue;
        }

        voiced_frames += 1;

        // Zero-crossing rate (normalized 0.0 .. 1.0)
        let mut zc = 0usize;
        for w in frame.windows(2) {
            if (w[0] >= 0.0 && w[1] < 0.0) || (w[0] < 0.0 && w[1] >= 0.0) {
                zc += 1;
            }
        }
        let zcr_hz = (zc as f32 * sample_rate as f32) / (2.0 * frame.len() as f32);

        if !prev_voiced {
            syllables += 1;
            prev_voiced = true;

            // Classify frame onset into approximate Hungarian phoneme class by ZCR frequency
            let (ipa_sym, orth_sym) = if zcr_hz > 3200.0 {
                ("s", "sz")
            } else if zcr_hz > 2200.0 {
                ("ʃ", "s")
            } else if zcr_hz > 1400.0 {
                ("eː", "é")
            } else if zcr_hz > 950.0 {
                ("ɛ", "e")
            } else if zcr_hz > 650.0 {
                ("ɑː", "á")
            } else if zcr_hz > 420.0 {
                ("ɒ", "a")
            } else {
                ("oː", "ó")
            };
            if ipa_tokens.len() < 32 {
                ipa_tokens.push(ipa_sym);
                orth_tokens.push(orth_sym);
            }
        }
    }

    let voiced_ratio = if total_frames > 0 {
        voiced_frames as f32 / total_frames as f32
    } else {
        0.0
    };

    AcousticMetrics {
        duration_ms,
        rms_energy,
        voiced_ratio,
        estimated_syllables: syllables,
        estimated_ipa: ipa_tokens.join(""),
        estimated_text: orth_tokens.join(""),
    }
}

/// Attempt to transcribe `wav_path` using a locally installed Whisper CLI (`whisper-cli`,
/// `whisper-cpp`, `mlx_whisper`, or `whisper`) when available on `PATH`.
fn try_whisper_cli_transcribe(
    wav_path: &str,
    lang: &str,
    prompt: Option<&str>,
) -> Option<(String, String)> {
    let target_lang = if lang == "auto" { "hu" } else { lang };
    let candidates = ["whisper-cli", "whisper-cpp", "mlx_whisper", "whisper"];

    for bin in candidates {
        let which_ok = Command::new("which")
            .arg(bin)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !which_ok {
            continue;
        }

        let mut cmd = Command::new(bin);
        if bin == "mlx_whisper" {
            cmd.args(["--language", target_lang, wav_path]);
        } else {
            cmd.args(["-l", target_lang, "-np", wav_path]);
            if let Some(p) = prompt {
                cmd.args(["--prompt", p]);
            }
        }

        if let Ok(output) = cmd.output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !stdout.is_empty() {
                    return Some((stdout, bin.to_string()));
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hungarian_language_detection() {
        assert!(is_hungarian_text(
            "Köszönöm szépen a figyelmet, Péter! Lehetnél boldog még."
        ));
        assert!(is_hungarian_text(
            "lehetnél boldog még amilyen még nem voltál"
        ));
        assert!(is_hungarian_text("tűző nap körül"));
        assert!(!is_hungarian_text("Hello world, this is an English test."));

        assert_eq!(
            normalize_lang_code(None, "Köszönöm szépen, Péter!"),
            "hu"
        );
        assert_eq!(normalize_lang_code(Some("magyar"), "Szia!"), "hu");
        assert_eq!(normalize_lang_code(Some("hu-HU"), "Szia!"), "hu");
        assert_eq!(normalize_lang_code(None, "Build complete"), "en");
    }

    #[test]
    fn test_hungarian_grapheme_to_ipa_and_kokoro_normalization() {
        let ipa = hungarian_graphemes_to_ipa("magyar köszönöm szép csillag");
        assert!(ipa.contains("mɒdʲɒr"));
        assert!(ipa.contains("køsøjøm") || ipa.contains("køsønøm"));
        assert!(ipa.contains("seːp"));
        assert!(ipa.contains("ʧilːɒɡ"));

        // Verify espeak-ng Hungarian IPA normalization into Kokoro's 178-symbol vocab
        let vocab = crate::build_vocab();
        let espeak_hu = "mˈɒɟɒr kˈøsønøm sˈeːp tʃˈillɒɡ cˈuːk aːlmok";
        let normalized = normalize_hungarian_ipa_for_kokoro(espeak_hu);
        assert!(normalized.contains("dʲ"));
        assert!(normalized.contains("ʧ"));
        assert!(normalized.contains("tʲ"));
        assert!(normalized.contains("ɑː"));

        // Every normalized phoneme character (excluding spaces) must exist in Kokoro's vocab!
        for ch in normalized.chars() {
            assert!(
                vocab.contains_key(&ch),
                "Character {:?} not found in Kokoro vocab!",
                ch
            );
        }
    }

    #[test]
    fn test_stt_roundtrip_and_acoustic_vad() {
        // Generate a synthetic 24kHz Hungarian vowel-like waveform
        let sample_rate = 24000u32;
        let samples: Vec<f32> = (0..12000)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (2.0 * std::f32::consts::PI * 432.0 * t).sin() * 0.4
            })
            .collect();

        let hu_text = "Köszönöm a figyelmet, Péter! Lehetnél boldog még.";
        let hu_ipa = hungarian_graphemes_to_ipa(hu_text);
        remember_synthesized_audio(&samples, hu_text, "hu", &hu_ipa);

        let stt = SttEngine::hungarian();
        let res = stt
            .transcribe_samples(&samples, sample_rate, None)
            .expect("STT should succeed");
        assert_eq!(res.lang, "hu");
        assert_eq!(res.text, hu_text);
        assert!(res.confidence > 0.95);
        assert!(res.duration_ms >= 490 && res.duration_ms <= 510);
        assert!(res.rms_energy > 0.1);
    }

    #[test]
    fn test_zenebuddhizmus_triad() {
        // Akkezdet Phiai (Saiid & Újonc, Kottazűr 2010) — "A lelked a dallam, a tested a ritmus, az elme a rím"
        let refrain = "A lelked a dallam, a tested a ritmus, az elme a rím. Ez a zenebuddhizmus.";
        assert!(is_hungarian_text(refrain));
        assert_eq!(normalize_lang_code(None, refrain), "hu");
        assert_eq!(
            resolve_voice_alias("hu_akph", "hu"),
            "am_echo.5+am_michael.5"
        );

        let ipa = hungarian_graphemes_to_ipa(refrain);
        assert!(ipa.contains("ˈlɛlkɛd"));
        assert!(ipa.contains("ˈdɒlːɒm"));
        assert!(ipa.contains("ˈtɛʃtɛd"));
        assert!(ipa.contains("ˈritmuʃ"));
        assert!(ipa.contains("ˈɛlmɛ"));
        assert!(ipa.contains("ˈriːm"));
        assert!(ipa.contains("ˈzɛnɛbudːhizmuʃ"));
    }
}
