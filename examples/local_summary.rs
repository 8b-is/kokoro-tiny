//! Offline, content-addressed summaries. Never plays audio automatically.
use kokoro_tiny::TtsEngine;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
};

fn hash_file(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let mut file = fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
fn key(text: &str) -> String {
    format!(
        "{:x}",
        Sha256::digest(format!("kokoro-summary-v1|af_sky|en-us|1.0|{text}").as_bytes())
    )
}
fn cached(wav: &Path, receipt: &Path, expected: &str) -> bool {
    let Ok(bytes) = fs::read(receipt) else {
        return false;
    };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return false;
    };
    value["key"].as_str() == Some(expected)
        && hash_file(wav).ok().as_deref() == value["wav_sha256"].as_str()
        && wav.is_file()
}
struct Lock(std::path::PathBuf);
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 5 {
        return Err("Usage: local_summary MODEL VOICES SUMMARY.txt OUTPUT_DIR".into());
    }
    // Limit accidental long inference requests before model loading.
    let mut bytes = Vec::new();
    fs::File::open(&args[3])?
        .take(4097)
        .read_to_end(&mut bytes)?;
    let text = String::from_utf8(bytes)?;
    let text = text.trim();
    if text.is_empty() || text.len() > 4096 {
        return Err("Summary must contain 1–4096 UTF-8 bytes".into());
    }
    let output = Path::new(&args[4]);
    fs::create_dir_all(output)?;
    let lock_path = output.join(".summary.lock");
    let mut lock = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock_path)?;
    let _guard = Lock(lock_path);
    writeln!(lock, "{}", std::process::id())?;
    let digest = key(text);
    let wav = output.join(format!("{digest}.wav"));
    let receipt = output.join(format!("{digest}.json"));
    if cached(&wav, &receipt, &digest) {
        println!("UNCHANGED {}", wav.display());
        return Ok(());
    }
    for (path, expected) in [
        (
            &args[1],
            "7d5df8ecf7d4b1878015a32686053fd0eebe2bc377234608764cc0ef3636a6c5",
        ),
        (
            &args[2],
            "bca610b8308e8d99f32e6fe4197e7ec01679264efed0cac9140fe9c29f1fbf7d",
        ),
    ] {
        if hash_file(Path::new(path))? != expected {
            return Err("Model hash mismatch; offline synthesis refused".into());
        }
    }
    let mut engine = TtsEngine::with_paths(&args[1], &args[2]).await?;
    if engine.voices().iter().any(|v| v == "fallback") {
        return Err("Fallback audio is not synthesis".into());
    }
    let audio = engine.synthesize(text, Some("af_sky"), Some(1.0), Some("en-us"))?;
    if audio.is_empty() || audio.iter().any(|v| !v.is_finite()) {
        return Err("Invalid audio".into());
    }
    let temporary = output.join(format!("{digest}.partial.wav"));
    engine.save_wav(temporary.to_str().ok_or("Invalid output path")?, &audio)?;
    fs::rename(&temporary, &wav)?;
    let record = serde_json::json!({"key":digest,"wav_sha256":hash_file(&wav)?,"samples":audio.len(),"fallback":false,"automatic_playback":false});
    let temporary_receipt = receipt.with_extension("json.partial");
    fs::write(&temporary_receipt, serde_json::to_vec_pretty(&record)?)?;
    fs::rename(temporary_receipt, receipt)?;
    println!("GENERATED {}", wav.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn changes_in_text_change_key() {
        assert_ne!(key("Done"), key("Still working"));
    }
    #[test]
    fn missing_or_modified_audio_is_not_cached() {
        let dir = std::env::temp_dir().join(format!("summary-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let wav = dir.join("sample.wav");
        let receipt = dir.join("receipt.json");
        fs::write(&wav, b"original").unwrap();
        fs::write(
            &receipt,
            serde_json::to_vec(
                &serde_json::json!({"key":"test","wav_sha256":hash_file(&wav).unwrap()}),
            )
            .unwrap(),
        )
        .unwrap();
        assert!(cached(&wav, &receipt, "test"));
        assert!(!cached(&wav, &receipt, "different"));
        fs::write(&wav, b"damaged").unwrap();
        assert!(!cached(&wav, &receipt, "test"));
        fs::remove_file(&wav).unwrap();
        assert!(!cached(&wav, &receipt, "test"));
        fs::remove_dir_all(dir).unwrap();
    }
}
