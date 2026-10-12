fn main() {
    let phonemes = espeak_rs::text_to_phonemes("ok ok", "en-us", None, true, false).unwrap();
    println!("{:?}", phonemes);
}
