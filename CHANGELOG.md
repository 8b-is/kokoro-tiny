# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Fixed
- Ensure phoneme sequences are padded with a pad token so short phrases (e.g., "It's 21:22") don't lose leading tokens during tokenization/inference. (PR #2)

### Changed
- Make `playback` an opt-in Cargo feature to avoid requiring system audio dev packages by default. (PR #2)

### CI
- Add GitHub Actions workflow for testing both default (no playback) and playback-enabled builds.

