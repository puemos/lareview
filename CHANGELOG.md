# Changelog

All notable changes to this project will be documented here. This project follows SemVer once we reach 1.0.

## [0.0.11] - 2025-12-24

### Added
- **Home View**: Introduced a new Home view as the default entry point, showing recent reviews and available agents.
- **Agent Discovery**: Added agent discovery and availability display on the Home view.
- **Typography**: Added centralized typography utilities and migrated UI text to consistent font helpers.
- **Icons**: Added Phosphor icon font integration and updated navigation icons.
- **Navigation**: Added Home navigation tab and routing support.

### Changed
- **Navigation**: Updated header navigation layout with dynamic tab sizing and centered container.
- **Default View**: Switched default app view from Generate to Home.
- **Theming**: Standardized spacing constants and adjusted header height; refined theme text colors for better visual hierarchy.
- **Reviews**: Updated review deletion flow to accept explicit review IDs and properly clear selection state.
- **Components**: Reworked multiple UI components to use shared typography and spacing utilities.

### Internal
- **GEMINI.md**: Added Git workflow guidelines.
- **CI**: Updated clippy invocation to include all targets and features.

## [0.0.10] - 2025-12-23

### Added
- **Generate View**: Enhanced generation timeline with better visual feedback.
- **Review View**: Added custom review selector dropdown with improved interactivity, sorting (selected first), and visual styling (brand colors, pointer cursors).

### Changed
- **UI/UX**: Unified status colors across the application:
    - `Todo` is now consistently gray (`text_muted`).
    - `Ignored` is now consistently red (`destructive`).
    - `InProgress` plan items now match the global accent color.
- **Assets**: Optimized application assets by converting screenshots to WebP format, reducing bundle size.
- **Documentation**: Updated README and Architecture docs; removed obsolete planning documents.
- **Review View**: Refined center pane layout and navigation tree styling.
- **Infra**: Updates to ACP task generator client and type definitions.

### Fixed
- **Review Selector**: Resolved issues with item clickability, hover states, and deprecated popup API usage.
- **Animations**: Improved cyber animation components.

## [0.0.9] - 2025-12-22

### Added

- Thread list component with status indicators and sorting by priority.
- Database migration system with migration 0009 for ReviewStatus constraint updates.
- LRU markdown caching for improved rendering performance during UI resize.
- Reusable list item component for consistent UI patterns.

### Changed

- Renamed `TaskStatus` to `ReviewStatus` for broader applicability across tasks and threads.
- Updated status enum variants: `Pending` → `Todo`, added `WIP` alias support.
- Review view now uses three-panel layout with resizable thread list sidebar.
- Improved markdown rendering with quantized width stepping to reduce layout thrashing.

### Fixed

- Database CHECK constraint now supports new status values (todo, in_progress, done, ignored) with backward compatibility.
- Performance issues during panel resize by caching parsed markdown structures.

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
