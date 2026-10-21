//! MCP (Model Context Protocol) Server for kokoro-tiny
//!
//! This module implements an MCP server that allows AI assistants like Claude
//! to use text-to-speech capabilities through a standardized JSON-RPC interface.
//!
//! # Tools Provided
//! - `speak_to_user`: Synthesize and play audio immediately
//! - `speak_with_emotion`: Auto-select voice based on emotion
//! - `list_voices`: Get all available voice presets
//! - `synthesize_to_file`: Save audio to file without playing

use crate::TtsEngine;
use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, Read, Write};

/// MCP Protocol version
const PROTOCOL_VERSION: &str = "2024-11-05";

/// MCP Request structure
#[derive(Debug, Deserialize)]
struct McpRequest {
    jsonrpc: String,
    id: Option<serde_json::Value>,
    method: String,
    params: Option<serde_json::Value>,
}

/// MCP Response structure
#[derive(Debug, Serialize)]
struct McpResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<McpError>,
}

/// MCP Error structure
#[derive(Debug, Serialize)]
struct McpError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
}

/// Tool definition for MCP
#[derive(Debug, Serialize)]
struct Tool {
    name: String,
    description: String,
    #[serde(rename = "inputSchema")]
    input_schema: serde_json::Value,
}

/// Emotion to voice mapping
fn emotion_to_voice(emotion: &str) -> &str {
    match emotion.to_lowercase().as_str() {
        "happy" | "cheerful" | "excited" | "success" => "af_bella",
        "alert" | "warning" | "error" | "serious" => "am_adam",
        "info" | "neutral" | "friendly" => "af_sky",
        "technical" | "precise" | "analytical" => "af_nicole",
        "professional" | "formal" | "teaching" => "am_michael",
        "warm" | "caring" | "encouraging" => "af_heart",
        "confident" | "announcement" => "am_echo",
        "british" | "polite" => "bf_emma",
        "robotic" | "system" => "af_sky", // Use default for now, will be robotic_sam later
        _ => "af_sky", // Default friendly voice
    }
}

/// MCP Server implementation
pub struct McpServer {
    tts: TtsEngine,
    stdin: io::StdinLock<'static>,
    stdout: io::Stdout,
}

impl McpServer {
    /// Create a new MCP server
    pub async fn new() -> Result<Self, String> {
        let tts = TtsEngine::new().await?;
        let stdin = Box::leak(Box::new(io::stdin())).lock();
        let stdout = io::stdout();
        
        Ok(Self { tts, stdin, stdout })
    }

    /// Run the MCP server main loop
    pub fn run(&mut self) -> Result<(), String> {
        eprintln!("🎤 Kokoro MCP Server starting...");
        eprintln!("📡 Protocol version: {}", PROTOCOL_VERSION);
        eprintln!("🔊 Ready to provide audio collaboration!");

        loop {
            let mut line = String::new();
            match self.stdin.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {},
                Err(e) => return Err(format!("Failed to read line: {}", e)),
            }
            
            if line.trim().is_empty() {
                continue;
            }

            let response = match serde_json::from_str::<McpRequest>(&line) {
                Ok(request) => self.handle_request(request),
                Err(e) => McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: None,
                    result: None,
                    error: Some(McpError {
                        code: -32700,
                        message: format!("Parse error: {}", e),
                        data: None,
                    }),
                },
            };

            let response_json = serde_json::to_string(&response)
                .map_err(|e| format!("Failed to serialize response: {}", e))?;
            
            writeln!(self.stdout, "{}", response_json)
                .map_err(|e| format!("Failed to write response: {}", e))?;
            
            self.stdout.flush()
                .map_err(|e| format!("Failed to flush stdout: {}", e))?;
        }

        Ok(())
    }

    /// Handle an MCP request
    fn handle_request(&mut self, request: McpRequest) -> McpResponse {
        eprintln!("📨 Received request: {}", request.method);

        let result = match request.method.as_str() {
            "initialize" => self.handle_initialize(&request),
            "tools/list" => self.handle_tools_list(),
            "tools/call" => self.handle_tools_call(&request),
            "ping" => Ok(serde_json::json!({"status": "ok"})),
            _ => Err(McpError {
                code: -32601,
                message: format!("Method not found: {}", request.method),
                data: None,
            }),
        };

        match result {
            Ok(result) => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(result),
                error: None,
            },
            Err(error) => McpResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: None,
                error: Some(error),
            },
        }
    }

    /// Handle initialize request
    fn handle_initialize(&self, _request: &McpRequest) -> Result<serde_json::Value, McpError> {
        Ok(serde_json::json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "kokoro-tiny-mcp",
                "version": "0.2.0"
            }
        }))
    }

    /// Handle tools/list request
    fn handle_tools_list(&self) -> Result<serde_json::Value, McpError> {
        let tools = vec![
            Tool {
                name: "speak_to_user".to_string(),
                description: "Speak a message to the user with text-to-speech. The audio will play immediately. Use this to get the user's attention, provide status updates, or give encouragement during collaboration.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "text": {
                            "type": "string",
                            "description": "The text to speak to the user"
                        },
                        "voice": {
                            "type": "string",
                            "description": "Voice preset to use (e.g., af_bella, am_adam, af_sky, hu_mariska, hu_peter). Optional, defaults to af_sky.",
                            "enum": ["af_sky", "af_bella", "af_nicole", "af_heart", "am_adam", "am_michael", "am_echo", "bf_emma", "bm_george", "hu_mariska", "hu_anna", "hu_eva", "hu_peter", "hu_bela", "hu_boldog"]
                        },
                        "lang": {
                            "type": "string",
                            "description": "Language code ('auto', 'en', 'hu' / 'magyar'). Optional, defaults to 'auto' (auto-detects Hungarian).",
                            "enum": ["auto", "en", "hu", "hu-HU", "magyar"]
                        },
                        "speed": {
                            "type": "number",
                            "description": "Speech speed (0.5 = slower, 1.0 = normal, 2.0 = faster). Optional, defaults to 1.0.",
                            "minimum": 0.5,
                            "maximum": 2.0
                        },
                        "volume": {
                            "type": "number",
                            "description": "Playback volume (0.0 to 1.0). Optional, defaults to 0.8.",
                            "minimum": 0.0,
                            "maximum": 1.0
                        }
                    },
                    "required": ["text"]
                }),
            },
            Tool {
                name: "speak_with_emotion".to_string(),
                description: "Speak a message with automatic voice selection based on emotion. The system will choose an appropriate voice for the given emotion.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "text": {
                            "type": "string",
                            "description": "The text to speak"
                        },
                        "emotion": {
                            "type": "string",
                            "description": "Emotion/context for voice selection",
                            "enum": ["happy", "alert", "info", "technical", "professional", "warm", "confident", "british", "robotic"]
                        },
                        "speed": {
                            "type": "number",
                            "description": "Speech speed. Optional, defaults to 1.0.",
                            "minimum": 0.5,
                            "maximum": 2.0
                        }
                    },
                    "required": ["text", "emotion"]
                }),
            },
            Tool {
                name: "list_voices".to_string(),
                description: "Get a list of all available voice presets with descriptions.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "synthesize_to_file".to_string(),
                description: "Synthesize text to speech and save to a file without playing it.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "text": {
                            "type": "string",
                            "description": "The text to synthesize"
                        },
                        "output_path": {
                            "type": "string",
                            "description": "Path where the audio file should be saved (e.g., /tmp/message.wav)"
                        },
                        "voice": {
                            "type": "string",
                            "description": "Voice preset to use. Optional, defaults to af_sky (or hu_mariska for Hungarian)."
                        },
                        "lang": {
                            "type": "string",
                            "description": "Language code ('auto', 'en', 'hu'). Optional, defaults to 'auto'."
                        },
                        "speed": {
                            "type": "number",
                            "description": "Speech speed. Optional, defaults to 1.0."
                        }
                    },
                    "required": ["text", "output_path"]
                }),
            },
            Tool {
                name: "speak_hungarian".to_string(),
                description: "Speak Hungarian (Magyar) text with Hungarian IPA phonemization and Magyar voice presets (`hu_mariska`, `hu_anna`, `hu_eva`, `hu_peter`, `hu_bela`, `hu_boldog`). Optionally saves to a WAV file.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "text": {
                            "type": "string",
                            "description": "Hungarian (Magyar) text to speak"
                        },
                        "voice": {
                            "type": "string",
                            "description": "Hungarian voice preset (`hu_mariska`, `hu_anna`, `hu_eva`, `hu_peter`, `hu_bela`, `hu_boldog`). Defaults to `hu_mariska`."
                        },
                        "speed": {
                            "type": "number",
                            "description": "Speech speed (0.5 to 2.0). Defaults to 1.0."
                        },
                        "output_path": {
                            "type": "string",
                            "description": "Optional WAV file path to save the synthesized Hungarian speech"
                        }
                    },
                    "required": ["text"]
                }),
            },
            Tool {
                name: "transcribe_audio".to_string(),
                description: "Base Speech-to-Text (STT) transcription from a WAV audio file, supporting Hungarian (`hu` / `hu-HU` / `magyar`) and English (`en`). Returns text, IPA phoneme stream, VAD speech ratio, and confidence.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "wav_path": {
                            "type": "string",
                            "description": "Path to the WAV file to transcribe"
                        },
                        "lang": {
                            "type": "string",
                            "description": "Target language code (`hu`, `en`, `auto`). Defaults to `hu`."
                        },
                        "prompt": {
                            "type": "string",
                            "description": "Optional vocabulary/context prompt for Hungarian or domain terms"
                        }
                    },
                    "required": ["wav_path"]
                }),
            },
        ];

        Ok(serde_json::json!({
            "tools": tools
        }))
    }

    /// Handle tools/call request
    fn handle_tools_call(&mut self, request: &McpRequest) -> Result<serde_json::Value, McpError> {
        let params = request.params.as_ref().ok_or_else(|| McpError {
            code: -32602,
            message: "Missing params".to_string(),
            data: None,
        })?;

        let tool_name = params.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError {
                code: -32602,
                message: "Missing tool name".to_string(),
                data: None,
            })?;

        let arguments = params.get("arguments")
            .ok_or_else(|| McpError {
                code: -32602,
                message: "Missing arguments".to_string(),
                data: None,
            })?;

        eprintln!("🔧 Calling tool: {}", tool_name);

        match tool_name {
            "speak_to_user" => self.tool_speak_to_user(arguments),
            "speak_with_emotion" => self.tool_speak_with_emotion(arguments),
            "speak_hungarian" => self.tool_speak_hungarian(arguments),
            "transcribe_audio" => self.tool_transcribe_audio(arguments),
            "list_voices" => self.tool_list_voices(),
            "synthesize_to_file" => self.tool_synthesize_to_file(arguments),
            _ => Err(McpError {
                code: -32602,
                message: format!("Unknown tool: {}", tool_name),
                data: None,
            }),
        }
    }

    /// Tool: speak_to_user
    fn tool_speak_to_user(&mut self, args: &serde_json::Value) -> Result<serde_json::Value, McpError> {
        let text = args.get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError {
                code: -32602,
                message: "Missing 'text' parameter".to_string(),
                data: None,
            })?;

        let voice = args.get("voice")
            .and_then(|v| v.as_str());

        let lang = args.get("lang")
            .and_then(|v| v.as_str());

        let speed = args.get("speed")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32;

        let volume = args.get("volume")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.8) as f32;

        let resolved_lang = crate::hungarian::normalize_lang_code(lang, text);
        eprintln!("🔊 Speaking [{}]: \"{}\" with voice {:?}", resolved_lang, text, voice);

        // Synthesize audio
        let audio = self.tts.synthesize_with_speed(text, voice, speed, Some(&resolved_lang))
            .map_err(|e| McpError {
                code: -32603,
                message: format!("Synthesis failed: {}", e),
                data: None,
            })?;

        // Play audio
        #[cfg(feature = "playback")]
        {
            self.tts.play(&audio, volume)
                .map_err(|e| McpError {
                    code: -32603,
                    message: format!("Playback failed: {}", e),
                    data: None,
                })?;
        }

        let duration_ms = (audio.len() as f32 / 24000.0 * 1000.0) as u32;

        let played = cfg!(feature = "playback");

        let status_text = if played {
            format!(
                "🔊 Spoke: \"{}\"\nVoice: {}\nDuration: {}ms",
                text,
                voice.unwrap_or("af_sky"),
                duration_ms
            )
        } else {
            format!(
                "💾 Synthesized (playback disabled): \"{}\"\nVoice: {}\nDuration: {}ms",
                text,
                voice.unwrap_or("af_sky"),
                duration_ms
            )
        };

        Ok(serde_json::json!({
            "content": [{
                "type": "text",
                "text": status_text
            }],
            "played": played,
            "duration_ms": duration_ms,
            "voice": voice.unwrap_or("af_sky")
        }))
    }

    /// Tool: speak_with_emotion
    fn tool_speak_with_emotion(&mut self, args: &serde_json::Value) -> Result<serde_json::Value, McpError> {
        let text = args.get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError {
                code: -32602,
                message: "Missing 'text' parameter".to_string(),
                data: None,
            })?;

        let emotion = args.get("emotion")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError {
                code: -32602,
                message: "Missing 'emotion' parameter".to_string(),
                data: None,
            })?;

        let speed = args.get("speed")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32;

        // Map emotion to voice
        let voice = emotion_to_voice(emotion);

        eprintln!("😊 Speaking with emotion '{}': voice={}", emotion, voice);

        // Synthesize and play
        let audio = self.tts.synthesize_with_speed(text, Some(voice), speed, None)
            .map_err(|e| McpError {
                code: -32603,
                message: format!("Synthesis failed: {}", e),
                data: None,
            })?;

        #[cfg(feature = "playback")]
        {
            self.tts.play(&audio, 0.8)
                .map_err(|e| McpError {
                    code: -32603,
                    message: format!("Playback failed: {}", e),
                    data: None,
                })?;
        }

        let duration_ms = (audio.len() as f32 / 24000.0 * 1000.0) as u32;

        Ok(serde_json::json!({
            "content": [{
                "type": "text",
                "text": format!("🔊 Spoke with emotion '{}': \"{}\"\nVoice: {}\nDuration: {}ms",
                    emotion,
                    text,
                    voice,
                    duration_ms
                )
            }],
            "played": true,
            "emotion": emotion,
            "voice": voice,
            "duration_ms": duration_ms
        }))
    }

    /// Tool: list_voices
    fn tool_list_voices(&self) -> Result<serde_json::Value, McpError> {
        let voice_descriptions = vec![
            ("af_sky", "Friendly American female (default)", "General purpose, status updates"),
            ("af_bella", "Cheerful American female", "Success messages, celebrations"),
            ("af_nicole", "Precise American female", "Technical content, analysis"),
            ("af_heart", "Warm American female", "Encouragement, personal messages"),
            ("am_adam", "Serious American male", "Alerts, errors, warnings"),
            ("am_michael", "Professional American male", "Formal content, teaching"),
            ("am_echo", "Confident American male", "Announcements, important info"),
            ("bf_emma", "Clear British female", "Warnings, polite messages"),
            ("bm_george", "British male", "Narration, storytelling"),
            ("hu_mariska", "Warm Hungarian female (Mariska tribute blend)", "Hungarian TTS default, warm Magyar speech"),
            ("hu_anna", "Clear Budapest female narration", "Hungarian articles, books, documentation"),
            ("hu_eva", "Gentle Hungarian female (Éva / Sanctuary)", "Poetic Hungarian, Sanctuary narration"),
            ("hu_peter", "Grounded Hungarian male (Péter blend)", "Hungarian male narration & dialogue"),
            ("hu_bela", "Confident Hungarian male baritone", "Announcements, alerts in Magyar"),
            ("hu_boldog", "Lyrical Hungarian voice (Boldog Még)", "Poetry, lyrics, heartfelt Hungarian messages"),
        ];

        let voices: Vec<_> = voice_descriptions.iter().map(|(name, desc, use_case)| {
            serde_json::json!({
                "name": name,
                "description": desc,
                "use_case": use_case
            })
        }).collect();

        Ok(serde_json::json!({
            "content": [{
                "type": "text",
                "text": format!("Available voices:\n{}", 
                    voice_descriptions.iter()
                        .map(|(n, d, u)| format!("• {}: {} ({})", n, d, u))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            }],
            "voices": voices,
            "count": voices.len()
        }))
    }

    /// Tool: synthesize_to_file
    fn tool_synthesize_to_file(&mut self, args: &serde_json::Value) -> Result<serde_json::Value, McpError> {
        let text = args.get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError {
                code: -32602,
                message: "Missing 'text' parameter".to_string(),
                data: None,
            })?;

        let output_path = args.get("output_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError {
                code: -32602,
                message: "Missing 'output_path' parameter".to_string(),
                data: None,
            })?;

        let voice = args.get("voice")
            .and_then(|v| v.as_str());

        let lang = args.get("lang")
            .and_then(|v| v.as_str());

        let speed = args.get("speed")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32;

        let resolved_lang = crate::hungarian::normalize_lang_code(lang, text);
        eprintln!("💾 Saving [{}] to file: {}", resolved_lang, output_path);

        // Synthesize audio
        let audio = self.tts.synthesize_with_speed(text, voice, speed, Some(&resolved_lang))
            .map_err(|e| McpError {
                code: -32603,
                message: format!("Synthesis failed: {}", e),
                data: None,
            })?;

        // Save to file
        self.tts.save_audio(output_path, &audio)
            .map_err(|e| McpError {
                code: -32603,
                message: format!("Failed to save file: {}", e),
                data: None,
            })?;

        let ipa = if resolved_lang == "hu" {
            crate::hungarian::hungarian_graphemes_to_ipa(text)
        } else {
            String::new()
        };
        crate::SttEngine::write_sidecar(output_path, text, &resolved_lang, &ipa);

        let duration_ms = (audio.len() as f32 / 24000.0 * 1000.0) as u32;

        Ok(serde_json::json!({
            "content": [{
                "type": "text",
                "text": format!("💾 Saved audio to: {}\nLanguage: {}\nDuration: {}ms\nVoice: {}",
                    output_path,
                    resolved_lang,
                    duration_ms,
                    voice.unwrap_or("af_sky")
                )
            }],
            "success": true,
            "path": output_path,
            "lang": resolved_lang,
            "ipa": ipa,
            "duration_ms": duration_ms,
            "voice": voice.unwrap_or("af_sky")
        }))
    }

    /// Tool: speak_hungarian
    fn tool_speak_hungarian(&mut self, args: &serde_json::Value) -> Result<serde_json::Value, McpError> {
        let text = args.get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError {
                code: -32602,
                message: "Missing 'text' parameter".to_string(),
                data: None,
            })?;

        let voice = args.get("voice")
            .and_then(|v| v.as_str())
            .unwrap_or("hu_mariska");

        let speed = args.get("speed")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as f32;

        let output_path = args.get("output_path")
            .and_then(|v| v.as_str());

        let audio = self.tts.synthesize_with_speed(text, Some(voice), speed, Some("hu"))
            .map_err(|e| McpError {
                code: -32603,
                message: format!("Hungarian synthesis failed: {}", e),
                data: None,
            })?;

        let ipa = crate::hungarian::hungarian_graphemes_to_ipa(text);

        if let Some(path) = output_path {
            self.tts.save_audio(path, &audio)
                .map_err(|e| McpError {
                    code: -32603,
                    message: format!("Failed to save file: {}", e),
                    data: None,
                })?;
            crate::SttEngine::write_sidecar(path, text, "hu", &ipa);
        }

        #[cfg(feature = "playback")]
        if output_path.is_none() {
            self.tts.play(&audio, 0.85)
                .map_err(|e| McpError {
                    code: -32603,
                    message: format!("Playback failed: {}", e),
                    data: None,
                })?;
        }

        let duration_ms = (audio.len() as f32 / 24000.0 * 1000.0) as u32;

        Ok(serde_json::json!({
            "content": [{
                "type": "text",
                "text": format!(
                    "🇭🇺 Magyar TTS: \"{}\"\nIPA: /{}/\nVoice: {}\nDuration: {}ms",
                    text, ipa, voice, duration_ms
                )
            }],
            "lang": "hu",
            "ipa": ipa,
            "voice": voice,
            "duration_ms": duration_ms,
            "output_path": output_path
        }))
    }

    /// Tool: transcribe_audio
    fn tool_transcribe_audio(&self, args: &serde_json::Value) -> Result<serde_json::Value, McpError> {
        let wav_path = args.get("wav_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError {
                code: -32602,
                message: "Missing 'wav_path' parameter".to_string(),
                data: None,
            })?;

        let lang = args.get("lang")
            .and_then(|v| v.as_str())
            .unwrap_or("hu");

        let prompt = args.get("prompt")
            .and_then(|v| v.as_str());

        let stt = crate::SttEngine::new();
        let mut opts = crate::SttOptions::default().lang(lang);
        if let Some(p) = prompt {
            opts = opts.prompt(p);
        }

        let res = stt.transcribe_wav(wav_path, Some(opts))
            .map_err(|e| McpError {
                code: -32603,
                message: format!("STT transcription failed: {}", e),
                data: None,
            })?;

        Ok(serde_json::json!({
            "content": [{
                "type": "text",
                "text": format!(
                    "🎧 STT [{} · {}]: \"{}\"\nIPA: /{}/\nDuration: {}ms · Confidence: {:.0}%",
                    res.lang,
                    res.backend,
                    res.text,
                    res.ipa,
                    res.duration_ms,
                    res.confidence * 100.0
                )
            }],
            "text": res.text,
            "lang": res.lang,
            "ipa": res.ipa,
            "confidence": res.confidence,
            "duration_ms": res.duration_ms,
            "rms_energy": res.rms_energy,
            "voiced_ratio": res.voiced_ratio,
            "estimated_syllables": res.estimated_syllables,
            "backend": res.backend
        }))
    }
}
