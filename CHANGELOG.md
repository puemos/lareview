# Changelog

All notable changes to this project will be documented here. This project follows SemVer once we reach 1.0.

## [0.0.3] - 2025-12-17

### Added
- Comprehensive installation guide with OS-specific instructions for macOS, Linux, and Windows
- Detailed steps for handling macOS security restrictions

### Changed
- Major overhaul of diff viewer architecture with better performance
- More efficient rendering with lazy loading and caching mechanism
- Refined header navigation with centered Generate/Review buttons
- Better accessibility with cursor icons
- Enhanced diagram rendering with improved D2 code handling
- Cleaner separation of concerns in the diff viewing system
- More efficient state management

### Fixed
- Various code organization and performance improvements

## Unreleased
- Added OSS hygiene docs (LICENSE, CONTRIBUTING, CODE_OF_CONDUCT, SECURITY).
- Expanded CI to lint all targets/features and added scheduled cargo-deny security checks.
- Documented nightly toolchain requirement for Rust 2024 edition.
- Hardened D2 installer flow with opt-in and copyable commands.
