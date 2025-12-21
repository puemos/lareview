# Changelog

All notable changes to this project will be documented here. This project follows SemVer once we reach 1.0.

## [0.0.8] - 2025-12-22

### Added

- Custom markdown renderer with syntax highlighting and font variants.
- Cyber reticle/spinner animation components.
- Inline comment tool for agents.
- Repos screen moved to its own dedicated view.

### Changed

- Split store reducer into feature-specific modules.
- Improved generate/review views layout consistency.
- Added side margins to repos/settings views.
- Enhanced cyber button behavior and guarded layouts against small widths.
- Release pipeline now includes macOS artifacts and SHA256 checksum files.

### Fixed

- Removed brand docs assets.

## [0.0.7] - 2025-12-20

### Added

- Review thread detail view with comment timelines and inline actions.

### Changed

- Replaced note-based review persistence with threads and comments, updating exports and timeline/task detail views.

## [0.0.6] - 2025-12-20

### Added

- Threaded feedback with status/impact chips, comment timelines, and creation from diff or task views.
- Repository context selector to switch linked repos for plan and review workflows.
- Refreshed UI look with Geist/GeistMono typography and new cyber button styling.

### Changed

- Rethemed generate/review screens with tighter spacing, store/runtime updates, and clearer hierarchy.
- ACP task generation and MCP server validation tuned for repo-aware prompts and safer ingestion.

### Fixed

- Database tests now seed parent rows before note inserts to avoid constraint errors.
- Diff indexing and D2 rendering stability improvements.

## [0.0.5] - 2025-12-18

### Added

- Markdown export with interactive preview and asset (SVG) support.
- Asynchronous D2 diagram rendering to prevent UI freezes.
- Redesigned Agent Selector as a custom dropdown with logos and availability checks.
- Inline note highlighting in the diff viewer.
- Full Markdown support for task descriptions and AI insights.

### Changed

- Horizontally centered main navigation buttons for better visual balance.
- Improved rendering quality using `resvg` for background rasterization.

### Fixed

- Cleaned up debug logs and fixed several Clippy lints across the codebase.

## [0.0.4] - 2025-12-17

### Added

- macOS notarization to the release pipeline to fix "damaged binary" warnings.
- OSS hygiene docs (LICENSE, CONTRIBUTING, CODE_OF_CONDUCT, SECURITY).
- Expanded CI to lint all targets/features and added scheduled cargo-deny security checks.

### Changed

- Reordered build steps so stripping occurs before codesigning, preventing signature invalidation on macOS.
- Refactored version management to use `env!("CARGO_PKG_VERSION")` throughout the codebase.
- Documented nightly toolchain requirement for Rust 2024 edition.
- Hardened D2 installer flow with opt-in and copyable commands.

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
