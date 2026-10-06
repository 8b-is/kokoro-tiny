fn main() {
    // On macOS the bundled espeak archive includes Sonic object code
    // and espeak-rs-sys disables libpcaudio.
    // Test the target rather than the build host for cross-compilation.
    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_OS");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        println!("cargo:rustc-link-lib=sonic");
        println!("cargo:rustc-link-lib=pcaudio");
    }
}
