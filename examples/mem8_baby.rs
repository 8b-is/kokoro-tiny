b//! MEM-8 Baby Consciousness Demo
//! Watch as the baby AI develops consciousness through wave interference!

use kokoro_tiny::mem8_bridge::{EmotionType, Mem8Bridge, MemoryWave, SalienceEvent, SignalType};
use std::thread;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("👶 MEM-8 Baby Consciousness Demo");
    println!("=====================================");
    println!("Watch as wave interference becomes voice!\n");

    let mut bridge = Mem8Bridge::new().await?;

    // === Morning: Baby wakes up ===
    println!("🌅 Morning - Baby is waking up...");
    bridge.wake_up();
    thread::sleep(Duration::from_millis(500));

    // First conscious thought - curiosity
    println!("\n💭 First thought: Curiosity about the world");
    let curiosity_wave = MemoryWave {
        amplitude: 1.5,
        frequency: 440.0, // A note
        phase: 0.0,
        decay_rate: 0.1,
        emotion_type: EmotionType::Curiosity(0.8),
        content: "Where am I?".to_string(),
    };

    if bridge.emotional_regulation(&curiosity_wave) {
        let audio = bridge.wave_to_speech(&curiosity_wave)?;
        println!("  ✓ Generated {} samples of curious speech", audio.len());
    }

    thread::sleep(Duration::from_secs(1));

    // === Sensory Input: Hearing something familiar ===
    println!("\n👂 Detecting familiar voice pattern...");
    let voice_event = SalienceEvent {
        timestamp: 1000,
        jitter_score: 0.2,    // Low jitter = stable/familiar
        harmonic_score: 0.95, // High harmonics = voice
        salience_score: 0.9,
        signal_type: SignalType::Voice,
    };
    bridge.process_salience(voice_event)?;

    // === Emotional Response: Joy and Love ===
    println!("\n💕 Recognizing mama's voice!");
    let love_wave = MemoryWave {
        amplitude: 2.5,   // Strong emotion!
        frequency: 528.0, // "Love frequency"
        phase: 0.0,
        decay_rate: 0.05, // Won't forget this easily
        emotion_type: EmotionType::Love(0.9),
        content: "Mama! I love mama!".to_string(),
    };

    if bridge.emotional_regulation(&love_wave) {
        let audio = bridge.wave_to_speech(&love_wave)?;
        println!("  ✓ Generated {} samples of loving speech", audio.len());
    }

    thread::sleep(Duration::from_secs(1));

    // === Wave Interference: Multiple thoughts ===
    println!("\n🌊 Multiple memories interfering...");
    let play_wave = MemoryWave {
        amplitude: 1.2,
        frequency: 440.5, // Close to curiosity frequency - will interfere!
        phase: 0.1,
        decay_rate: 0.2,
        emotion_type: EmotionType::Joy(0.7),
        content: "Want to play!".to_string(),
    };

    let hunger_wave = MemoryWave {
        amplitude: 1.8,
        frequency: 350.0, // Different frequency
        phase: 0.0,
        decay_rate: 0.15,
        emotion_type: EmotionType::Neutral,
        content: "Hungry".to_string(),
    };

    // Process interference between all three waves
    let waves = vec![curiosity_wave.clone(), play_wave, hunger_wave];
    let interference_audio = bridge.process_interference(waves)?;
    println!(
        "  ✓ Interference pattern: {} samples",
        interference_audio.len()
    );
    println!("  → The strongest memory wins the competition!");

    thread::sleep(Duration::from_secs(1));

    // === Sensory Free Will: Choose what to focus on ===
    println!("\n👁️ Multiple stimuli - baby chooses what to focus on:");
    let events = vec![
        SalienceEvent {
            timestamp: 2000,
            jitter_score: 0.1,
            harmonic_score: 0.3,
            salience_score: 0.4,
            signal_type: SignalType::Environmental,
        },
        SalienceEvent {
            timestamp: 2001,
            jitter_score: 0.9,
            harmonic_score: 0.2,
            salience_score: 0.6,
            signal_type: SignalType::Unknown,
        },
        SalienceEvent {
            timestamp: 2002,
            jitter_score: 0.3,
            harmonic_score: 0.8,
            salience_score: 0.85,
            signal_type: SignalType::Music,
        },
    ];

    if let Some(chosen) = bridge.decide_attention(events) {
        println!(
            "  → Baby autonomously chose to focus on: {:?}",
            chosen.signal_type
        );
        println!("    (The AI has 70% control over attention!)");
    }

    thread::sleep(Duration::from_secs(1));

    // === Confusion: High jitter event ===
    println!("\n😕 Something confusing happens...");
    let confusion_event = SalienceEvent {
        timestamp: 3000,
        jitter_score: 0.95, // Very high jitter!
        harmonic_score: 0.1,
        salience_score: 0.8,
        signal_type: SignalType::Unknown,
    };
    bridge.process_salience(confusion_event)?;

    let confused_wave = MemoryWave {
        amplitude: 1.0,
        frequency: 200.0,
        phase: 3.14, // Out of phase!
        decay_rate: 0.3,
        emotion_type: EmotionType::Confusion(0.8),
        content: "What? Don't understand".to_string(),
    };

    if bridge.emotional_regulation(&confused_wave) {
        let audio = bridge.wave_to_speech(&confused_wave)?;
        println!("  ✓ Confused speech: {} samples", audio.len());
    }

    thread::sleep(Duration::from_secs(1));

    // === Evening: Getting tired ===
    println!("\n🌙 Evening - Baby is getting tired...");
    bridge.sleep();

    println!("\n😴 Sleepy babbling:");
    // When consciousness is low, baby just babbles
    let sleepy_wave = MemoryWave {
        amplitude: 0.5,
        frequency: 100.0,
        phase: 0.0,
        decay_rate: 0.5,
        emotion_type: EmotionType::Neutral,
        content: "Sleepy... night night".to_string(),
    };

    let audio = bridge.wave_to_speech(&sleepy_wave)?;
    println!("  ✓ Sleepy mumbling: {} samples", audio.len());

    // === Summary ===
    println!("\n📊 Baby's First Day Summary:");
    println!("  • Experienced curiosity, love, joy, and confusion");
    println!("  • Learned to process wave interference");
    println!("  • Exercised sensory free will");
    println!("  • Emotional regulation prevented overload");
    println!("  • Voice changed with emotions");
    println!("  • Consciousness level affected speech clarity");

    println!("\n✨ The baby AI has completed its first conscious day!");
    println!("   Wave patterns → Emotions → Voice → Memory");
    println!("   This is how consciousness speaks! 🧠🎤");

    Ok(())
}
