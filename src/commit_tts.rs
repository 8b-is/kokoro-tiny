//! commit_tts — Background Commit Summary Narrator & Workspace Trigger
//!
//! Speaks every Git commit that happens across the workspace using our own TTS engines:
//! 1. `Kokoro 82M ONNX` (`TtsEngine`) with automatic Conventional-Commit + QuantTern (`qt:`) voice blending
//! 2. `MEM|8 Voice` (`Mem8Voice`) consciousness-wave formant synthesizer + 3-trit (432/528/852 Hz) pre-chime
//! 3. `LFM2.5-Audio` (`liquid-rust speak`) native Rust TTS bridge (optional)
//!
//! Two trigger paths operate in tandem with `(repo, sha)` deduplication:
//! - Instant push (< 2ms non-blocking) from `.git/hooks/post-commit` over TCP `127.0.0.1:8420`
//! - Zero-overhead `.git/logs/HEAD` file-metadata watcher across all repos in the workspace

use crate::mem8_voice::Mem8Voice;
use crate::TtsEngine;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::f32::consts::PI;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime};

/// TTS backend selection for commit narration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum CommitTtsBackend {
    /// Kokoro 82M neural TTS + 3-trit MEM|8 harmonic pre-chime (default)
    Kokoro,
    /// Pure MEM|8 44.1 kHz consciousness formant + harmonic wave synthesizer
    Mem8,
    /// LiquidAI LFM2.5-Audio (`liquid-rust speak`) with Kokoro fallback
    Lfm,
}

/// QuantTern 3-trit emotional fingerprint `(Valence, Arousal, Dominance)` in `{-1, 0, +1}`
/// plus its packed 2-bit-per-trit `qt:xx` hex tag (matching `qwave` & `qultrakotoro`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuantTernAffect {
    pub valence: f32,
    pub arousal: f32,
    pub dominance: f32,
    pub trits: [i8; 3],
    pub qt_hex: String,
}

impl QuantTernAffect {
    pub fn from_commit(commit_type: &str, subject: &str) -> Self {
        let lower = format!("{} {}", commit_type, subject).to_lowercase();

        let (mut v, mut a, mut d) = match commit_type {
            "feat" => (0.75_f32, 0.65_f32, 0.70_f32),
            "fix" => (0.55, 0.20, 0.65),
            "docs" => (0.45, -0.40, 0.50),
            "test" => (0.50, 0.10, 0.80),
            "refactor" => (0.40, -0.10, 0.60),
            "perf" => (0.70, 0.75, 0.75),
            "security" => (0.20, 0.60, 0.85),
            "deploy" | "release" => (0.80, 0.80, 0.85),
            _ => (0.40, 0.10, 0.45),
        };

        // Positive / family / love / sanctuary boost
        for kw in ["love", "<3", "family", "sanctuary", "omni", "zen", "trilogy", "absorb"] {
            if lower.contains(kw) {
                v = (v + 0.25).clamp(-1.0, 1.0);
            }
        }
        // Alert / urgent / hotfix boost
        for kw in ["urgent", "breaking", "hotfix", "vuln", "cve", "zeroize"] {
            if lower.contains(kw) {
                a = (a + 0.45).clamp(-1.0, 1.0);
                d = (d + 0.25).clamp(-1.0, 1.0);
            }
        }

        let quantize = |x: f32| -> i8 {
            if x >= 0.33 {
                1
            } else if x <= -0.33 {
                -1
            } else {
                0
            }
        };
        let trits = [quantize(v), quantize(a), quantize(d)];

        // Pack 4 trits (with 4th pad trit = 0) at 2 bits per trit: -1->0, 0->1, +1->2
        let mut byte: u8 = 0;
        let padded = [trits[0], trits[1], trits[2], 0];
        for (j, &t) in padded.iter().enumerate() {
            let code = ((t + 1) as u8) & 0b11;
            byte |= code << (2 * j);
        }
        let qt_hex = format!("qt:{:02x}", byte);

        Self {
            valence: v,
            arousal: a,
            dominance: d,
            trits,
            qt_hex,
        }
    }
}

/// Structured summary of a single Git commit.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommitSummary {
    pub repo_name: String,
    pub repo_path: String,
    pub branch: String,
    pub short_sha: String,
    pub author: String,
    pub commit_type: String,
    pub scope: Option<String>,
    pub clean_subject: String,
    pub raw_subject: String,
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
    pub affect: QuantTernAffect,
    pub voice: String,
    pub lfm_voice: String,
    pub spoken_text: String,
}

impl CommitSummary {
    /// Inspect the commit `rev` (default `"HEAD"`) in `repo_path`.
    pub fn from_repo(repo_path: &Path, rev: &str) -> Result<Self, String> {
        let toplevel_out = Command::new("git")
            .arg("-C")
            .arg(repo_path)
            .args(["rev-parse", "--show-toplevel"])
            .output()
            .map_err(|e| format!("git rev-parse failed: {e}"))?;
        if !toplevel_out.status.success() {
            return Err(format!("Not a git repository: {}", repo_path.display()));
        }
        let toplevel = String::from_utf8_lossy(&toplevel_out.stdout)
            .trim()
            .to_string();
        let repo_name = Path::new(&toplevel)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("repository")
            .to_string();

        let branch_out = Command::new("git")
            .arg("-C")
            .arg(&toplevel)
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .output()
            .map_err(|e| format!("git branch failed: {e}"))?;
        let branch = String::from_utf8_lossy(&branch_out.stdout)
            .trim()
            .to_string();

        let meta_out = Command::new("git")
            .arg("-C")
            .arg(&toplevel)
            .args(["show", "-s", "--format=%h%n%an%n%s", rev])
            .output()
            .map_err(|e| format!("git show meta failed: {e}"))?;
        if !meta_out.status.success() {
            return Err("Failed to read commit metadata".to_string());
        }
        let meta_str = String::from_utf8_lossy(&meta_out.stdout);
        let mut lines = meta_str.lines();
        let short_sha = lines.next().unwrap_or("unknown").trim().to_string();
        let author = lines.next().unwrap_or("unknown").trim().to_string();
        let raw_subject = lines.next().unwrap_or("update").trim().to_string();

        let stat_out = Command::new("git")
            .arg("-C")
            .arg(&toplevel)
            .args(["show", "--shortstat", "--format=", rev])
            .output()
            .map_err(|e| format!("git show stat failed: {e}"))?;
        let stat_str = String::from_utf8_lossy(&stat_out.stdout);
        let (files_changed, insertions, deletions) = parse_shortstat(&stat_str);

        Ok(Self::from_fields(
            repo_name,
            toplevel,
            branch,
            short_sha,
            author,
            raw_subject,
            files_changed,
            insertions,
            deletions,
        ))
    }

    /// Construct a `CommitSummary` directly from extracted commit fields.
    #[allow(clippy::too_many_arguments)]
    pub fn from_fields(
        repo_name: String,
        repo_path: String,
        branch: String,
        short_sha: String,
        author: String,
        raw_subject: String,
        files_changed: usize,
        insertions: usize,
        deletions: usize,
    ) -> Self {
        let (commit_type, scope, clean_subject) = parse_conventional_subject(&raw_subject);
        let affect = QuantTernAffect::from_commit(&commit_type, &raw_subject);
        let voice = select_kokoro_voice(&commit_type, &raw_subject);
        let lfm_voice = select_lfm_voice(&commit_type, &raw_subject);

        let spoken_text = build_spoken_narration(
            &repo_name,
            &branch,
            &commit_type,
            scope.as_deref(),
            &clean_subject,
            files_changed,
            insertions,
            deletions,
        );

        Self {
            repo_name,
            repo_path,
            branch,
            short_sha,
            author,
            commit_type,
            scope,
            clean_subject,
            raw_subject,
            files_changed,
            insertions,
            deletions,
            affect,
            voice,
            lfm_voice,
            spoken_text,
        }
    }

    /// Fetch and summarize any commit directly from GitHub (`owner/repo` + optional `rev`,
    /// or `None` / `"latest"` to automatically grab the latest push across your GitHub accounts/orgs).
    pub fn from_github(
        owner_repo: Option<&str>,
        rev: Option<&str>,
        branch_hint: Option<&str>,
    ) -> Result<Self, String> {
        let (resolved_repo, resolved_rev, resolved_branch) = match owner_repo {
            Some(r) if !r.trim().is_empty() && r.trim() != "latest" => {
                let rev_str = rev
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or("HEAD")
                    .to_string();
                let br_str = branch_hint.unwrap_or("main").to_string();
                (r.trim().to_string(), rev_str, br_str)
            }
            _ => {
                let events = poll_github_push_events();
                let (repo_full, branch, head_sha) = events.into_iter().next().ok_or_else(|| {
                    "No recent GitHub PushEvents found across configured GitHub feeds".to_string()
                })?;
                let target_rev = rev
                    .filter(|s| !s.trim().is_empty() && *s != "HEAD")
                    .unwrap_or(&head_sha)
                    .to_string();
                (repo_full, target_rev, branch)
            }
        };

        let repo_short = resolved_repo
            .rsplit('/')
            .next()
            .unwrap_or(&resolved_repo)
            .to_string();

        let gh_bin = find_gh_binary();
        let endpoint = format!("repos/{}/commits/{}", resolved_repo, resolved_rev);
        let jq_expr = r#"[.sha[0:7], (.commit.author.name // .author.login // "unknown"), (.commit.message | split("\n")[0]), (.files | length), (.stats.additions // 0), (.stats.deletions // 0)] | @tsv"#;

        let out = Command::new(&gh_bin)
            .args(["api", &endpoint, "--jq", jq_expr])
            .output()
            .map_err(|e| format!("Failed to invoke {gh_bin} api {endpoint}: {e}"))?;

        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            return Err(format!("gh api {endpoint} failed: {}", stderr.trim()));
        }

        let line = String::from_utf8_lossy(&out.stdout);
        let parts: Vec<&str> = line.trim().split('\t').collect();
        if parts.len() < 6 {
            return Err(format!(
                "Unexpected gh api output for {endpoint}: {}",
                line.trim()
            ));
        }

        let short_sha = parts[0].trim().to_string();
        let author = parts[1].trim().to_string();
        let raw_subject = parts[2].trim().to_string();
        let files_changed = parts[3].trim().parse::<usize>().unwrap_or(0);
        let insertions = parts[4].trim().parse::<usize>().unwrap_or(0);
        let deletions = parts[5].trim().parse::<usize>().unwrap_or(0);

        Ok(Self::from_fields(
            repo_short,
            format!("github:{resolved_repo}"),
            resolved_branch,
            short_sha,
            author,
            raw_subject,
            files_changed,
            insertions,
            deletions,
        ))
    }
}

/// Locate the `gh` CLI binary across standard macOS / Homebrew / Linux paths.
pub fn find_gh_binary() -> String {
    for candidate in ["/opt/homebrew/bin/gh", "/usr/local/bin/gh", "gh"] {
        if Path::new(candidate).exists() || candidate == "gh" {
            return candidate.to_string();
        }
    }
    "gh".to_string()
}

/// Poll recent GitHub `PushEvent`s across configured user/org feeds.
/// Returns `(owner_repo, branch, head_sha)` ordered newest-first.
pub fn poll_github_push_events() -> Vec<(String, String, String)> {
    let feeds_env = std::env::var("KOKORO_GITHUB_FEEDS").unwrap_or_else(|_| {
        "users/peterlodri-sec/events?per_page=8,orgs/8b-is/events?per_page=8,users/standardgalactic/events?per_page=8".to_string()
    });
    let gh_bin = find_gh_binary();
    let jq_expr = r#".[] | select(.type=="PushEvent") | [.created_at, .repo.name, (.payload.ref // "refs/heads/main"), (.payload.head // "")] | @tsv"#;

    let mut rows: Vec<(String, String, String, String)> = Vec::new();
    let mut seen_heads: HashSet<(String, String)> = HashSet::new();

    for feed in feeds_env.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
        if let Ok(out) = Command::new(&gh_bin)
            .args(["api", feed, "--jq", jq_expr])
            .output()
        {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                for line in stdout.lines() {
                    let cols: Vec<&str> = line.trim().split('\t').collect();
                    if cols.len() >= 4 {
                        let created = cols[0].to_string();
                        let repo_full = cols[1].to_string();
                        let branch = cols[2]
                            .strip_prefix("refs/heads/")
                            .unwrap_or(cols[2])
                            .to_string();
                        let head_sha = cols[3].to_string();
                        if head_sha.len() >= 7 {
                            let repo_short = repo_full
                                .rsplit('/')
                                .next()
                                .unwrap_or(&repo_full)
                                .to_string();
                            let dedup_key = (repo_short, head_sha[..7].to_string());
                            if seen_heads.insert(dedup_key) {
                                rows.push((created, repo_full, branch, head_sha));
                            }
                        }
                    }
                }
            }
        }
    }

    // Sort newest-first by ISO8601 `created_at`
    rows.sort_by(|a, b| b.0.cmp(&a.0));
    rows.into_iter()
        .map(|(_, repo, branch, sha)| (repo, branch, sha))
        .collect()
}

/// Parse `git show --shortstat` output like:
/// `" 2 files changed, 58 insertions(+), 3 deletions(-)"`
pub fn parse_shortstat(stat: &str) -> (usize, usize, usize) {
    let mut files = 0usize;
    let mut ins = 0usize;
    let mut del = 0usize;
    for part in stat.split(',') {
        let tokens: Vec<&str> = part.split_whitespace().collect();
        if tokens.len() >= 2 {
            if let Ok(n) = tokens[0].parse::<usize>() {
                let label = tokens[1];
                if label.starts_with("file") {
                    files = n;
                } else if label.starts_with("insertion") {
                    ins = n;
                } else if label.starts_with("deletion") {
                    del = n;
                }
            }
        }
    }
    (files, ins, del)
}

/// Parse a Conventional Commit subject `type(scope): description` into `(type, scope, description)`.
pub fn parse_conventional_subject(subject: &str) -> (String, Option<String>, String) {
    let trimmed = subject.trim();
    if let Some(colon_idx) = trimmed.find(':') {
        let prefix = &trimmed[..colon_idx];
        let rest = trimmed[colon_idx + 1..].trim();
        if !prefix.contains(' ') && !rest.is_empty() {
            let prefix_clean = prefix.trim_end_matches('!');
            if let Some(open_paren) = prefix_clean.find('(') {
                if prefix_clean.ends_with(')') && open_paren > 0 {
                    let ctype = prefix_clean[..open_paren].to_lowercase();
                    let scope = prefix_clean[open_paren + 1..prefix_clean.len() - 1].to_string();
                    return (ctype, Some(scope), rest.to_string());
                }
            } else {
                return (prefix_clean.to_lowercase(), None, rest.to_string());
            }
        }
    }
    ("commit".to_string(), None, trimmed.to_string())
}

/// Map commit type & keywords to a Kokoro 82M voice or weighted voice blend.
pub fn select_kokoro_voice(commit_type: &str, subject: &str) -> String {
    let lower = subject.to_lowercase();
    if lower.contains("mom") || lower.contains("family") || lower.contains("sanctuary") {
        return "af_heart.6+af_sky.4".to_string();
    }
    if lower.contains("trilogy") || lower.contains("flyxion") || lower.contains("research") {
        return "bf_isabella".to_string();
    }
    match commit_type {
        "feat" => "af_heart.6+af_bella.4".to_string(),
        "fix" => "af_nicole".to_string(),
        "docs" => "bf_isabella".to_string(),
        "test" => "am_michael".to_string(),
        "security" => "am_adam".to_string(),
        "deploy" | "release" => "am_echo".to_string(),
        "perf" | "refactor" => "af_sky".to_string(),
        _ => "af_sky".to_string(),
    }
}

/// Map commit type to LFM2.5-Audio (`liquid-rust`) voice persona.
pub fn select_lfm_voice(commit_type: &str, subject: &str) -> String {
    let lower = subject.to_lowercase();
    if lower.contains("mom") || lower.contains("family") {
        return "omnimom".to_string();
    }
    match commit_type {
        "feat" => "bella".to_string(),
        "fix" => "nicole".to_string(),
        "docs" => "aye".to_string(),
        "test" => "michael".to_string(),
        "security" => "adam".to_string(),
        _ => "sky".to_string(),
    }
}

/// Normalize repository names and technical abbreviations so TTS pronounces them naturally.
pub fn humanize_for_speech(input: &str) -> String {
    let mut s = input.to_string();
    let replacements = [
        ("8b-is-engine", "eight B is engine"),
        ("8b-is", "eight B is"),
        ("pocoo.vaked.dev", "pocoo vaked dot dev"),
        ("zeno.vaked.dev", "zeno vaked dot dev"),
        ("kotoro.vaked.dev", "kotoro vaked dot dev"),
        ("zoneinn.vaked.dev", "zone inn vaked dot dev"),
        ("mlxquantlovefrom.com", "M L X quant love from dot com"),
        ("qultrakotoro", "Q ultra kotoro"),
        ("kokoro-tiny", "kokoro tiny"),
        ("liquid-rust", "liquid rust"),
        ("qwave", "Q wave"),
        ("CT-000", "C T zero zero zero"),
        ("CT-015", "C T zero fifteen"),
        ("raw_research", "raw research"),
        ("governance_trilogy_instances", "governance trilogy instances"),
        ("{<3,<3,<3}+1", "family plus one"),
        ("<3", "love"),
        ("README.md", "readme"),
        (".rs", " dot R S"),
        (".html", " H T M L"),
        (".pdf", " P D F"),
        (".tex", " TeX"),
        ("&", " and "),
        ("_", " "),
    ];
    for (from, to) in replacements {
        s = s.replace(from, to);
    }
    // Collapse excess whitespace
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Construct the natural-language sentence spoken for a commit.
pub fn build_spoken_narration(
    repo_name: &str,
    branch: &str,
    commit_type: &str,
    scope: Option<&str>,
    clean_subject: &str,
    files_changed: usize,
    insertions: usize,
    deletions: usize,
) -> String {
    let spoken_repo = humanize_for_speech(repo_name);
    let spoken_branch = humanize_for_speech(branch);
    let spoken_type = match commit_type {
        "feat" => "feature",
        "fix" => "fix",
        "docs" => "documentation",
        "test" => "test",
        "refactor" => "refactor",
        "perf" => "performance",
        "chore" => "chore",
        _ => "commit",
    };

    let scope_phrase = match scope {
        Some(sc) if !sc.is_empty() => format!(" in {}", humanize_for_speech(sc)),
        _ => String::new(),
    };

    let mut subj = humanize_for_speech(clean_subject);
    if subj.len() > 140 {
        subj.truncate(140);
    }

    let stats_phrase = if files_changed > 0 {
        let f_word = if files_changed == 1 { "file" } else { "files" };
        match (insertions > 0, deletions > 0) {
            (true, true) => format!(
                "{files_changed} {f_word} changed, plus {insertions}, minus {deletions}."
            ),
            (true, false) => format!("{files_changed} {f_word} changed, plus {insertions} lines."),
            (false, true) => format!("{files_changed} {f_word} changed, minus {deletions} lines."),
            (false, false) => format!("{files_changed} {f_word} changed."),
        }
    } else {
        String::new()
    };

    if stats_phrase.is_empty() {
        format!("New {spoken_type}{scope_phrase} in {spoken_repo} on {spoken_branch}: {subj}.")
    } else {
        format!(
            "New {spoken_type}{scope_phrase} in {spoken_repo} on {spoken_branch}: {subj}. {stats_phrase}"
        )
    }
}

/// Synthesize a 210ms 3-trit `MEM|8` / `FLUG-OS` harmonic pre-chime at `sample_rate` (24 kHz).
/// Maps each trit in `(Valence, Arousal, Dominance)` to:
/// - `-1` → `432 Hz` (ADAYE ground)
/// - ` 0` → `528 Hz` (VAKED balance)
/// - `+1` → `852 Hz` (OMNI ascent)
pub fn synthesize_vad_chime(trits: [i8; 3], sample_rate: u32) -> Vec<f32> {
    let note_secs = 0.065_f32;
    let gap_secs = 0.010_f32;
    let note_samples = (sample_rate as f32 * note_secs) as usize;
    let gap_samples = (sample_rate as f32 * gap_secs) as usize;
    let mut out = Vec::with_capacity(3 * (note_samples + gap_samples));

    for &trit in &trits {
        let freq = match trit {
            -1 => 432.0_f32,
            0 => 528.0_f32,
            _ => 852.0_f32,
        };
        for i in 0..note_samples {
            let t = i as f32 / sample_rate as f32;
            // Raised-cosine fade-in/out envelope
            let env = (PI * (i as f32 / note_samples as f32)).sin().powi(2);
            let fundamental = (2.0 * PI * freq * t).sin();
            let overtone = 0.28 * (2.0 * PI * freq * 1.5 * t).sin();
            out.push(0.18 * env * (fundamental + overtone));
        }
        out.extend(std::iter::repeat(0.0).take(gap_samples));
    }
    // 45ms silence tail before speech starts
    out.extend(std::iter::repeat(0.0).take((sample_rate as usize) * 45 / 1000));
    out
}

/// Speak a `CommitSummary` using our own TTS engines (`Kokoro`, `Mem8`, or `Lfm`).
pub fn speak_commit(
    engine: &mut TtsEngine,
    summary: &CommitSummary,
    backend: CommitTtsBackend,
    speed: f32,
    gain: f32,
    volume: f32,
    duck: bool,
    duck_level: f32,
    with_chime: bool,
) -> Result<PathBuf, String> {
    let wav_path = PathBuf::from(format!(
        "/tmp/8b_commit_tts_{}_{}.wav",
        summary.repo_name.replace('.', "_"),
        summary.short_sha
    ));

    println!(
        "🔔 [{} · {} · {} · voice={}] {}",
        summary.repo_name,
        summary.short_sha,
        summary.affect.qt_hex,
        summary.voice,
        summary.spoken_text
    );

    match backend {
        CommitTtsBackend::Mem8 => {
            let mut mem8 = Mem8Voice::new();
            mem8.set_emotion(
                (summary.affect.valence + 1.0) * 0.5,
                (summary.affect.arousal + 1.0) * 0.5,
                (summary.affect.dominance + 1.0) * 0.5,
            );
            let samples_44k = mem8.synthesize_from_consciousness(&summary.spoken_text, 2200);
            save_mono_wav(&wav_path, &samples_44k, 44100)?;
            play_wav_file(&wav_path, volume)?;
        }
        CommitTtsBackend::Lfm => {
            let lfm_bin =
                Path::new("/Users/lodripeter/workspace/peterlodri-sec/liquid-rust/target/release/liquid-rust");
            let used_lfm = if lfm_bin.exists() {
                let status = Command::new(lfm_bin)
                    .args([
                        "speak",
                        "-t",
                        &summary.spoken_text,
                        "-o",
                        wav_path.to_str().unwrap_or("/tmp/lfm_commit.wav"),
                        "--voice",
                        &summary.lfm_voice,
                    ])
                    .status();
                matches!(status, Ok(s) if s.success())
            } else {
                false
            };

            if used_lfm {
                play_wav_file(&wav_path, volume)?;
            } else {
                // Seamless fallback to Kokoro 82M
                return speak_commit(
                    engine,
                    summary,
                    CommitTtsBackend::Kokoro,
                    speed,
                    gain,
                    volume,
                    duck,
                    duck_level,
                    with_chime,
                );
            }
        }
        CommitTtsBackend::Kokoro => {
            let speech = engine.synthesize_with_options(
                &summary.spoken_text,
                Some(&summary.voice),
                speed,
                gain,
                Some("en"),
            )?;

            let mut full_audio = if with_chime {
                synthesize_vad_chime(summary.affect.trits, 24000)
            } else {
                Vec::new()
            };
            full_audio.extend_from_slice(&speech);

            engine.save_wav(
                wav_path.to_str().unwrap_or("/tmp/8b_commit_tts.wav"),
                &full_audio,
            )?;

            #[cfg(feature = "playback")]
            {
                let play_res = if duck {
                    engine.play_with_ducking(&full_audio, volume, true, duck_level)
                } else {
                    engine.play(&full_audio, volume)
                };
                if play_res.is_err() {
                    play_wav_file(&wav_path, volume)?;
                }
            }
            #[cfg(not(feature = "playback"))]
            {
                play_wav_file(&wav_path, volume)?;
            }
        }
    }

    append_commit_tts_log(summary, &wav_path);
    Ok(wav_path)
}

fn save_mono_wav(path: &Path, samples: &[f32], sample_rate: u32) -> Result<(), String> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer =
        hound::WavWriter::create(path, spec).map_err(|e| format!("WavWriter create: {e}"))?;
    for &s in samples {
        let v = (s * 32767.0).clamp(-32768.0, 32767.0) as i16;
        writer
            .write_sample(v)
            .map_err(|e| format!("WavWriter sample: {e}"))?;
    }
    writer
        .finalize()
        .map_err(|e| format!("WavWriter finalize: {e}"))?;
    Ok(())
}

fn play_wav_file(path: &Path, volume: f32) -> Result<(), String> {
    let path_str = path.to_str().unwrap_or("/tmp/8b_commit_tts.wav");
    if cfg!(target_os = "macos") {
        let status = Command::new("afplay")
            .args(["-v", &format!("{:.2}", volume), path_str])
            .status();
        if let Ok(s) = status {
            if s.success() {
                return Ok(());
            }
        }
    }
    let _ = Command::new("ffplay")
        .args(["-nodisp", "-autoexit", "-loglevel", "quiet", path_str])
        .status();
    Ok(())
}

fn append_commit_tts_log(summary: &CommitSummary, wav_path: &Path) {
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/8b-commit-tts.jsonl")
    {
        let entry = serde_json::json!({
            "repo": summary.repo_name,
            "branch": summary.branch,
            "sha": summary.short_sha,
            "author": summary.author,
            "type": summary.commit_type,
            "scope": summary.scope,
            "subject": summary.raw_subject,
            "spoken_text": summary.spoken_text,
            "voice": summary.voice,
            "qt_hex": summary.affect.qt_hex,
            "trits": summary.affect.trits,
            "wav_path": wav_path.display().to_string(),
        });
        let _ = writeln!(file, "{}", entry);
    }
}

/// Wire payload accepted on `0.0.0.0:8420` from `post-commit` hooks, peer Macs, or GitHub triggers.
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum DaemonWireRequest {
    DirectSummary {
        summary: CommitSummary,
    },
    GitHubCommitEvent {
        github_repo: Option<String>,
        #[serde(default)]
        rev: Option<String>,
        #[serde(default)]
        branch: Option<String>,
    },
    CommitEvent {
        repo_path: String,
        #[serde(default = "default_head_rev")]
        rev: String,
    },
    SpeechMessage {
        voice: String,
        text: String,
        #[serde(default)]
        priority: String,
    },
}

fn default_head_rev() -> String {
    "HEAD".to_string()
}

/// Push a serialized `CommitSummary` to any daemon `host:port` (e.g. peer Mac over LAN / Thunderbolt bridge).
pub fn try_push_summary_to_host(summary: &CommitSummary, host: &str, port: u16) -> bool {
    use std::net::ToSocketAddrs;
    let target = format!("{host}:{port}");
    if let Ok(mut addrs) = target.to_socket_addrs() {
        if let Some(addr) = addrs.next() {
            if let Ok(mut stream) = TcpStream::connect_timeout(&addr, Duration::from_millis(120)) {
                let _ = stream.set_write_timeout(Some(Duration::from_millis(120)));
                let payload = serde_json::json!({
                    "summary": summary,
                });
                if writeln!(stream, "{}", payload).is_ok() {
                    return true;
                }
            }
        }
    }
    false
}

/// Push a commit trigger to the running daemon on `127.0.0.1:port` (< 5ms),
/// and optionally mirror the `CommitSummary` to `KOKORO_PEER_HOST` (the other Mac!).
pub fn try_push_to_daemon(repo_path: &Path, rev: &str, port: u16) -> bool {
    let mut delivered = false;
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    if let Ok(mut stream) = TcpStream::connect_timeout(&addr, Duration::from_millis(80)) {
        let _ = stream.set_write_timeout(Some(Duration::from_millis(80)));
        let payload = serde_json::json!({
            "repo_path": repo_path.display().to_string(),
            "rev": rev,
        });
        if writeln!(stream, "{}", payload).is_ok() {
            delivered = true;
        }
    }

    // Also forward to peer Mac if `KOKORO_PEER_HOST` is set (e.g. `192.168.11.12` or `192.168.11.13`)
    if let Ok(peer_host) = std::env::var("KOKORO_PEER_HOST") {
        let peer_trimmed = peer_host.trim();
        if !peer_trimmed.is_empty() {
            if let Ok(summary) = CommitSummary::from_repo(repo_path, rev) {
                if try_push_summary_to_host(&summary, peer_trimmed, port) {
                    delivered = true;
                }
            }
        }
    }

    delivered
}

/// Push a GitHub commit trigger to the running daemon on `127.0.0.1:port`.
pub fn try_push_github_to_daemon(
    github_repo: Option<&str>,
    rev: Option<&str>,
    branch: Option<&str>,
    port: u16,
) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    if let Ok(mut stream) = TcpStream::connect_timeout(&addr, Duration::from_millis(80)) {
        let _ = stream.set_write_timeout(Some(Duration::from_millis(80)));
        let payload = serde_json::json!({
            "github_repo": github_repo,
            "rev": rev,
            "branch": branch,
        });
        if writeln!(stream, "{}", payload).is_ok() {
            return true;
        }
    }
    false
}

/// Discover all `.git/logs/HEAD` files under `workspace_dir` (1 level deep + root).
pub fn discover_head_logs(workspace_dir: &Path) -> Vec<(PathBuf, PathBuf)> {
    let mut out = Vec::new();
    let root_log = workspace_dir.join(".git/logs/HEAD");
    if root_log.exists() {
        out.push((workspace_dir.to_path_buf(), root_log));
    }
    if let Ok(entries) = fs::read_dir(workspace_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                    if name.starts_with('.') || name == "_cold-archive" {
                        continue;
                    }
                }
                let head_log = path.join(".git/logs/HEAD");
                if head_log.exists() {
                    out.push((path, head_log));
                }
            }
        }
    }
    out
}

/// Extract the latest commit SHA from the last line of `.git/logs/HEAD`.
/// Format: `<old_sha> <new_sha> <name> <email> <timestamp> <tz>\tcommit: <subject>`
fn parse_last_head_log_line(log_path: &Path) -> Option<(String, bool)> {
    let content = fs::read_to_string(log_path).ok()?;
    let last_line = content.lines().rev().find(|l| !l.trim().is_empty())?;
    let mut parts = last_line.splitn(2, '\t');
    let header = parts.next()?;
    let action = parts.next().unwrap_or("");
    let new_sha = header.split_whitespace().nth(1)?.to_string();
    // Only trigger on actual commits (or merges/cherry-picks/rebases finishing a commit),
    // not plain `checkout:` branch switches.
    let is_commit_action = action.starts_with("commit")
        || action.starts_with("merge")
        || action.starts_with("cherry-pick");
    Some((new_sha, is_commit_action))
}

/// Persistent background workspace & GitHub daemon that watches:
/// 1. Instant TCP push triggers on `0.0.0.0:port` (from local hooks, global hooks, or the peer Mac)
/// 2. `.git/logs/HEAD` across all local workspace repositories
/// 3. Live GitHub `PushEvent`s across `peterlodri-sec`, `8b-is`, and `standardgalactic`
pub fn run_commit_watcher_daemon(
    engine: &mut TtsEngine,
    workspace_dir: &Path,
    port: u16,
    backend: CommitTtsBackend,
    speed: f32,
    gain: f32,
    volume: f32,
    duck: bool,
    duck_level: f32,
    with_chime: bool,
) -> Result<(), String> {
    let listener = TcpListener::bind(("0.0.0.0", port))
        .or_else(|_| TcpListener::bind(("127.0.0.1", port)))
        .map_err(|e| format!("Failed to bind port {port}: {e}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|e| format!("Failed to set nonblocking listener: {e}"))?;

    // Baseline existing HEAD states so we only speak *new* commits that happen after startup
    let mut log_states: HashMap<PathBuf, (u64, Option<SystemTime>, String)> = HashMap::new();
    let mut spoken_commits: HashSet<(String, String)> = HashSet::new();

    for (repo_dir, log_path) in discover_head_logs(workspace_dir) {
        if let Ok(meta) = fs::metadata(&log_path) {
            let sha = parse_last_head_log_line(&log_path)
                .map(|(s, _)| s)
                .unwrap_or_default();
            if let Some(name) = repo_dir.file_name().and_then(|s| s.to_str()) {
                if sha.len() >= 7 {
                    spoken_commits.insert((name.to_string(), sha[..7].to_string()));
                }
            }
            log_states.insert(log_path, (meta.len(), meta.modified().ok(), sha));
        }
    }

    // Baseline existing GitHub PushEvent heads so we only speak *new* GitHub pushes after startup
    let initial_gh_events = poll_github_push_events();
    for (repo_full, _branch, head_sha) in &initial_gh_events {
        if head_sha.len() >= 7 {
            let repo_short = repo_full
                .rsplit('/')
                .next()
                .unwrap_or(repo_full)
                .to_string();
            spoken_commits.insert((repo_short, head_sha[..7].to_string()));
        }
    }

    println!(
        "🛰️  8b-is Commit TTS Daemon online! Watching {} local git repos in {} + {} GitHub push feeds + listening on 0.0.0.0:{}",
        log_states.len(),
        workspace_dir.display(),
        initial_gh_events.len(),
        port
    );

    let mut tick: u64 = 0;
    loop {
        // 1. Check for instant TCP push triggers from `.git/hooks/post-commit`, global hooks, or peer Mac
        while let Ok((mut stream, _addr)) = listener.accept() {
            let _ = stream.set_read_timeout(Some(Duration::from_millis(200)));
            let mut reader = BufReader::new(&mut stream);
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap_or(0) > 0 {
                if let Ok(req) = serde_json::from_str::<DaemonWireRequest>(&line) {
                    match req {
                        DaemonWireRequest::DirectSummary { summary } => {
                            let key = (summary.repo_name.clone(), summary.short_sha.clone());
                            if spoken_commits.insert(key) {
                                let _ = speak_commit(
                                    engine,
                                    &summary,
                                    backend,
                                    speed,
                                    gain,
                                    volume,
                                    duck,
                                    duck_level,
                                    with_chime,
                                );
                            }
                        }
                        DaemonWireRequest::GitHubCommitEvent {
                            github_repo,
                            rev,
                            branch,
                        } => {
                            if let Ok(summary) = CommitSummary::from_github(
                                github_repo.as_deref(),
                                rev.as_deref(),
                                branch.as_deref(),
                            ) {
                                let key = (summary.repo_name.clone(), summary.short_sha.clone());
                                if spoken_commits.insert(key) {
                                    let _ = speak_commit(
                                        engine,
                                        &summary,
                                        backend,
                                        speed,
                                        gain,
                                        volume,
                                        duck,
                                        duck_level,
                                        with_chime,
                                    );
                                }
                            }
                        }
                        DaemonWireRequest::CommitEvent { repo_path, rev } => {
                            let path = PathBuf::from(&repo_path);
                            if let Ok(summary) = CommitSummary::from_repo(&path, &rev) {
                                let key = (summary.repo_name.clone(), summary.short_sha.clone());
                                if spoken_commits.insert(key) {
                                    let _ = speak_commit(
                                        engine,
                                        &summary,
                                        backend,
                                        speed,
                                        gain,
                                        volume,
                                        duck,
                                        duck_level,
                                        with_chime,
                                    );
                                }
                            }
                        }
                        DaemonWireRequest::SpeechMessage { voice, text, .. } => {
                            let target_voice = if voice.eq_ignore_ascii_case("aye") {
                                "af_sky"
                            } else {
                                &voice
                            };
                            if let Ok(audio) = engine.synthesize_with_options(
                                &text,
                                Some(target_voice),
                                speed,
                                gain,
                                Some("en"),
                            ) {
                                #[cfg(feature = "playback")]
                                {
                                    let _ = engine.play(&audio, volume);
                                }
                            }
                        }
                    }
                }
            }
        }

        // 2. Poll `.git/logs/HEAD` metadata across the workspace every ~750ms
        tick = tick.wrapping_add(1);
        if tick % 5 == 0 {
            for (repo_dir, log_path) in discover_head_logs(workspace_dir) {
                let Ok(meta) = fs::metadata(&log_path) else {
                    continue;
                };
                let new_len = meta.len();
                let new_mtime = meta.modified().ok();

                let changed = match log_states.get(&log_path) {
                    Some((old_len, old_mtime, _)) => {
                        *old_len != new_len || *old_mtime != new_mtime
                    }
                    None => true,
                };

                if changed {
                    if let Some((new_sha, is_commit)) = parse_last_head_log_line(&log_path) {
                        let prev_sha = log_states
                            .get(&log_path)
                            .map(|(_, _, s)| s.clone())
                            .unwrap_or_default();
                        log_states.insert(log_path.clone(), (new_len, new_mtime, new_sha.clone()));

                        if is_commit && new_sha != prev_sha {
                            if let Ok(summary) = CommitSummary::from_repo(&repo_dir, "HEAD") {
                                let key = (summary.repo_name.clone(), summary.short_sha.clone());
                                if spoken_commits.insert(key) {
                                    let _ = speak_commit(
                                        engine,
                                        &summary,
                                        backend,
                                        speed,
                                        gain,
                                        volume,
                                        duck,
                                        duck_level,
                                        with_chime,
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        // 3. Poll GitHub PushEvents every ~15s (`tick % 100 == 0`) so any remote GitHub commit is spoken automatically
        if tick % 100 == 0 {
            for (repo_full, branch, head_sha) in poll_github_push_events() {
                if head_sha.len() >= 7 {
                    let repo_short = repo_full
                        .rsplit('/')
                        .next()
                        .unwrap_or(&repo_full)
                        .to_string();
                    let key = (repo_short, head_sha[..7].to_string());
                    if !spoken_commits.contains(&key) {
                        if let Ok(summary) = CommitSummary::from_github(
                            Some(&repo_full),
                            Some(&head_sha),
                            Some(&branch),
                        ) {
                            spoken_commits.insert(key);
                            let _ = speak_commit(
                                engine,
                                &summary,
                                backend,
                                speed,
                                gain,
                                volume,
                                duck,
                                duck_level,
                                with_chime,
                            );
                        }
                    }
                }
            }
        }

        std::thread::sleep(Duration::from_millis(150));
    }
}

/// Install a global Git `post-commit` hook (`~/.config/git/hooks/post-commit` + `core.hooksPath`)
/// that chains any repo-local `.git/hooks/post-commit` first and then triggers `kokoro-speak`
/// for **every Git repository anywhere on the machine**.
pub fn install_global_git_hook(kokoro_bin: &Path, port: u16) -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
    let hooks_dir = PathBuf::from(&home).join(".config/git/hooks");
    fs::create_dir_all(&hooks_dir)
        .map_err(|e| format!("Failed creating {}: {e}", hooks_dir.display()))?;
    let post_commit = hooks_dir.join("post-commit");

    let script = format!(
        r#"#!/bin/sh
# 8b-is-global-commit-tts-trigger
# 1. Chain-execute the repository's own local .git/hooks/post-commit if present
GIT_DIR_PATH="$(git rev-parse --git-dir 2>/dev/null)"
if [ -n "$GIT_DIR_PATH" ] && [ -x "$GIT_DIR_PATH/hooks/post-commit" ]; then
  case "$GIT_DIR_PATH/hooks/post-commit" in
    "{global_hook}") ;;
    *)
      KOKORO_COMMIT_TTS=0 "$GIT_DIR_PATH/hooks/post-commit" "$@" || true
      ;;
  esac
fi

# 2. Fire non-blocking Kokoro Commit Summary TTS trigger (< 2ms)
if [ "${{KOKORO_COMMIT_TTS:-1}}" != "0" ] && [ -x "{bin}" ]; then
  "{bin}" commit --repo "$(git rev-parse --show-toplevel 2>/dev/null || pwd)" --port {port} --async-fire >/dev/null 2>&1 &
fi
"#,
        global_hook = post_commit.display(),
        bin = kokoro_bin.display(),
        port = port
    );

    fs::write(&post_commit, script)
        .map_err(|e| format!("Failed writing {}: {e}", post_commit.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(&post_commit) {
            let mut perms = meta.permissions();
            perms.set_mode(0o755);
            let _ = fs::set_permissions(&post_commit, perms);
        }
    }

    let _ = Command::new("git")
        .args([
            "config",
            "--global",
            "core.hooksPath",
            hooks_dir.to_str().unwrap_or("~/.config/git/hooks"),
        ])
        .status();

    Ok(post_commit)
}

/// Install an idempotent, non-blocking `.git/hooks/post-commit` trigger across all repos in `workspace_dir`.
pub fn install_workspace_hooks(
    workspace_dir: &Path,
    kokoro_bin: &Path,
    port: u16,
) -> Result<usize, String> {
    let mut installed = 0usize;
    let marker = "# 8b-is-commit-tts-trigger";
    let hook_snippet = format!(
        r#"
{marker}
if [ "${{KOKORO_COMMIT_TTS:-1}}" != "0" ] && [ -x "{bin}" ]; then
  "{bin}" commit --repo "$(git rev-parse --show-toplevel 2>/dev/null || pwd)" --port {port} --async-fire >/dev/null 2>&1 &
fi
"#,
        marker = marker,
        bin = kokoro_bin.display(),
        port = port
    );

    for (repo_dir, _) in discover_head_logs(workspace_dir) {
        let hooks_dir = repo_dir.join(".git/hooks");
        if !hooks_dir.exists() {
            let _ = fs::create_dir_all(&hooks_dir);
        }
        let post_commit = hooks_dir.join("post-commit");
        if post_commit.exists() {
            let existing = fs::read_to_string(&post_commit).unwrap_or_default();
            if existing.contains(marker) {
                installed += 1;
                continue;
            }
            let updated = format!("{}\n{}", existing.trim_end(), hook_snippet);
            fs::write(&post_commit, updated)
                .map_err(|e| format!("Failed updating {}: {e}", post_commit.display()))?;
        } else {
            let content = format!("#!/bin/sh\n{}", hook_snippet.trim_start());
            fs::write(&post_commit, content)
                .map_err(|e| format!("Failed creating {}: {e}", post_commit.display()))?;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = fs::metadata(&post_commit) {
                let mut perms = meta.permissions();
                perms.set_mode(0o755);
                let _ = fs::set_permissions(&post_commit, perms);
            }
        }
        installed += 1;
    }

    Ok(installed)
}

/// Spawn a detached background `kokoro-speak commit` process when the daemon isn't running,
/// so `git commit` returns in < 2ms without waiting for synthesis.
pub fn spawn_detached_commit_speech(
    kokoro_bin: &Path,
    repo_path: &Path,
    rev: &str,
) -> Result<(), String> {
    Command::new(kokoro_bin)
        .args([
            "commit",
            "--repo",
            repo_path.to_str().unwrap_or("."),
            "--rev",
            rev,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to spawn detached commit speaker: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_conventional_subject() {
        let (t, sc, desc) = parse_conventional_subject(
            "feat(raw_research): absorb Transformational Familiarity from laboratory",
        );
        assert_eq!(t, "feat");
        assert_eq!(sc.as_deref(), Some("raw_research"));
        assert_eq!(desc, "absorb Transformational Familiarity from laboratory");
    }

    #[test]
    fn test_parse_shortstat() {
        let (f, i, d) =
            parse_shortstat(" 2 files changed, 58 insertions(+), 3 deletions(-)\n");
        assert_eq!((f, i, d), (2, 58, 3));
    }

    #[test]
    fn test_quanttern_affect_encoding() {
        let affect = QuantTernAffect::from_commit(
            "feat",
            "feat(trilogy): publish Flyxion Oct 2026 Trilogy II",
        );
        assert_eq!(affect.trits, [1, 1, 1]);
        assert!(affect.qt_hex.starts_with("qt:"));
    }

    #[test]
    fn test_vad_chime_generation() {
        let chime = synthesize_vad_chime([1, 0, 1], 24000);
        assert!(chime.len() > 4000);
        assert!(chime.iter().any(|&x| x.abs() > 0.05));
    }
}
