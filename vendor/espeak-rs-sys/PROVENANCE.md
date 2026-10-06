# Vendored espeak-rs-sys 0.1.9

Source: published crates.io package espeak-rs-sys 0.1.9.
Archive SHA-256: `eeb333310ae915d57961a6bbc67f53ef7b313a9ee80d393d55c0f27f1b65c848`.
Upstream repository recorded by the package: https://github.com/thewh1teagle/piper-rs
Package VCS revision: b574061c0755dcd88aa68e7b7992dedee5ae3a74.

The only source modification is build.rs: CMAKE_BUILD_PARALLEL_LEVEL now honors Cargo NUM_JOBS, with a fallback of two workers. All other original package files were compared byte-for-byte with the local registry source. Cargo cache markers are not included.

The Rust package declares MIT in Cargo.toml. The bundled eSpeak NG sources carry their own GPL notices, which remain intact; COPYING.espeak-ng contains the GPLv3 text from https://raw.githubusercontent.com/espeak-ng/espeak-ng/master/COPYING. Additional Unicode and component notices remain in their source directories. This directory does not relicense those components under kokoro-tiny's Apache license.

This local integration is not a crates.io release. Review downstream packaging and license obligations before distributing binaries or publishing the crate; Cargo's patch override applies only at this checkout root.
