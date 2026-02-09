# Changelog

All notable changes to this project will be documented here. This project follows SemVer once we reach 1.0.

## [0.0.33] - 2026-02-09

### Added

- **UI**: Update modal showing changelog and brew update command.

## [0.0.32] - 2026-02-09

### Fixed

- **Shell**: Always apply all environment variables from shell (removed conditional logic that skipped non-PATH vars).
- **GitLab**: Extract login from glab CLI output even on partial failure (e.g., multiple instances configured).
- **GitLab**: Added comprehensive tests for MR reference parsing.

## [0.0.31] - 2026-02-04

### Fixed

- **Release**: Fixed version mismatch between Cargo.toml and tauri.conf.json that caused macOS artifacts to be uploaded to wrong release.

## [0.0.30] - 2026-02-03

### Added

- **Review**: Merge confidence scoring system (1-5 scale) with agent-provided assessment and reasons.
- **UI**: MergeConfidenceBadge component with tooltip showing score, label, and assessment details.
- **Export**: Include merge confidence in markdown exports and VCS push requests.

## [0.0.28] - 2026-01-23

### Added

- **Release**: Linux tarball artifact (`lareview-linux.tar.gz`) for simplified binary distribution.

## [0.0.27] - 2026-01-21

### Added

- **Learning**: Learning patterns system for tracking and applying code review insights.
- **Learning**: Learning view UI with pattern management and rejection tracking.
- **Learning**: Confidence threshold configuration in settings.
- **Platform**: WSL support for CLI installation and URL opening.
- **Review**: Category badges and confidence display on feedback items.
- **Review**: Show uncovered files in review summary.
- **UI**: Card and MarkdownRenderer components for improved content display.

### Changed

- **Task Generator**: Updated prompts and MCP server for better agent output.
- **Rules**: Improved rules UI and library modal.

### Fixed

- **Task Generator**: Warn instead of error for uncovered files.

## [0.0.26] - 2026-01-20

### Added

- **Rules**: Rule library with curated review rules to import into reviews.
- **Review**: Issue checks system and summary dashboard (FilesHeatmap, IssueChecklist, KeyFeedback, TaskFlow).

### Changed

- **UI**: Improved loading states with dedicated view skeletons and smoother data transitions.

### Fixed

- **UI**: Minor React warnings and quote escaping issues.

## [0.0.25] - 2026-01-20

### Added

- **Prompt**: Strategic review analysis section (change type, blast radius, author intent).
- **Prompt**: Explicit "don't comment on" guidance to reduce low-value feedback noise.

### Changed

- **Prompt**: Restructured review process into understand→organize→review→submit phases.
- **Prompt**: Simplified and consolidated redundant sections for clarity.
- **Prompt**: Updated examples to use `line_id` instead of deprecated `line_content`.
- **Tasks**: Removed mandatory task coverage enforcement (all files no longer required to be covered).

### Fixed

- **Prompt**: Typo `ai_insignt_format` → `ai_insight_format`.
- **Prompt**: Inconsistent terminology ("diagram JSON" → "mermaid diagram").

## [0.0.24] - 2026-01-16

### Added

- **Generation**: Implement 'always allow snapshot' preference for smoother agent workflow.

### Changed

- **Plan**: Simplified plan management to strictly follow ACP compliance, removing redundant frontend logic.

### Fixed

- **UI**: Fixed `PrInput` layout and sizing issues in the Generate view.
- **UI**: Respect manual plan panel collapse state across plan updates.

### VCS

- **VCS**: Ensure git fetch is performed before snapshot creation to handle remote-only commits.

## [0.0.23] - 2026-01-16

### Added

- **Diagrams**: Enhanced zoom, fullscreen, and error handling with sanitization.
- **Settings**: GitLab branding and granular refresh.
- **UI**: Gutter menu with 'open in editor' and UX improvements.
- **VCS**: Support for GitLab Merge Requests alongside GitHub PRs.
- **Repos**: Allow cloning and linking GitLab repositories for MR reviews.
- **Generation**: Let agents read code via temporary snapshots with user consent.

### Changed

- **Review**: Remove task tab slide-in animations.
- **Tasks**: Enforce full file coverage by failing finalize when changes are uncovered.
- **Generation**: Rename worktree to snapshot and safely defer repo read permissions.
- **Internal**: Centralize LaReview tool names for consistent auto-approval.
- **Feedback**: Standardize progress events from comment to feedback terminology.

### Fixed

- **Tests**: Prevent `fake_acp_agent` from being included in the release bundle.
- **UI**: Improve feedback button hit area and styling.
- **Frontend**: Resolve lint warnings and stabilize async generation tests.

## [0.0.22] - 2026-01-10

### Added

- **Rules**: Added comprehensive Rules system for code reviews.
- **UI/UX**: Implemented interactive Rule popovers and hover tooltips.
- **Generation**: Fixed state persistence issues during agent generation.
- **Database**: New migrations and repository support for Rule-based feedback.
- **Integration**: Added Rule ingestion support to MCP server.

## [0.0.21] - 2026-01-09

### Added

- **Generation**: Implemented stop functionality for agent generation.
- **Generate**: Added real-time diff validation.
- **UI/UX**: Implemented minimalist toast notifications (Sonner) for user feedback.
- **UI/UX**: Added premium micro-interactions and refactored animation system.

### Changed

- **Generate**: Refined alert UI for better visual feedback.
- **Docs**: Updated release process documentation.

## [0.0.20] - 2026-01-09

### Added

- **Feedback**: Added global and line-specific feedback support.
- **UI/UX**: Custom confirmation modals for feedback and review deletion.
- **UI/UX**: Improved plan UI with Phosphor icons and synchronized status tracking.
- **UI/UX**: Risk level icons with tooltips in task lists and headers.
- **Review**: Auto-selection of the first change when switching tasks.
- **Settings**: Ability to modify AI agent arguments in settings.

### Changed

- **UI/UX**: Refined feedback interaction and improved diff viewer reliability.
- **UI/UX**: Enhanced feedback reply design and visual contrast.
- **UI/UX**: Standardized markdown styling to `prose-sm` for better readability.

### Fixed

- **Frontend**: Fixed `react-markdown` dependency placement.
- **Diff**: Improved handling of new files in the diff viewer.

## [0.0.19] - 2026-01-08

### Added

- **CI/CD**: Added Homebrew formula and cask auto-bump job to release workflow using `brew` CLI

## [0.0.18] - 2026-01-07

### Added

- **Tauri 2 Architecture**: Complete migration from egui to Tauri 2 with React frontend
- **React Frontend**: New modern UI built with React, TanStack Query, custom hooks, and Vite
- **Agent Icons**: New icons for Claude, Codex, Gemini, Grok, Kimi, Mistral, OpenCode, and Qwen
- **Landing Page**: New dedicated landing page (`landing/`) with marketing assets and SEO optimization
- **Tauri Capabilities**: Desktop and macOS capability schemas for permission management

### Changed

- **UI Framework**: Migrated from egui to Tauri 2 with React frontend
- **Command System**: New unified Tauri command handlers (`src/commands/mod.rs`)
- **Project Structure**: Separated frontend into dedicated `frontend/` directory with Vite
- **Build Pipeline**: Updated CI/CD for Tauri builds, codesigning, and artifact generation

### Removed

- **egui UI**: Removed entire `src/ui/` directory
- **Legacy CLI**: Removed standalone `src/bin/lareview_cli.rs` (integrated into main binary)
- **Old Templates**: Removed `src/prompts/mod.rs` (moved `generate_tasks.hbs` to `src/`)

## [0.0.17] - 2026-01-02

### Added

- **Terminal Workflow CLI**: New `lareview` CLI binary for launching the GUI with pre-loaded diffs:
  - `lareview main feature` - compare branches
  - `lareview pr owner/repo#123` - review GitHub PRs
  - `lareview --status` - review uncommitted changes
  - `git diff | lareview --stdin` - pipe diff to GUI
- **Syntax Highlighting**: New syntect-based syntax highlighting for diff viewer with LRU caching
- **CLI Settings View**: New settings page showing CLI installation status and usage examples
- **Domain Errors**: New `domain/error.rs` with typed errors (ReviewError, TaskError, FeedbackError, etc.)
- **Structured Logging**: Environment variable support via `RUST_LOG` (e.g., `RUST_LOG=debug cargo run`)
- **Double-Click Window**: Double-click header to maximize/restore window

### Changed

- **UI**: Frameless window with native vibrancy and full-size content view
- **Landing**: Refreshed landing page with new screenshots and feature descriptions
- **Diff Viewer**: Reduced cache size (2000→500 lines) and overscan (200→50) for faster rendering
- **Repo Linking**: Async and sync repo linking for CLI handoff support
- **Logging**: Switched to structured logging with RUST_LOG support

### Removed

- **Utils**: Removed `utils/os.rs` and `window.rs` (simplified window handling)

## [0.0.16] - 2026-01-01

### Added

- **GitHub PR Feedback**: Added batch feedback submission to GitHub PRs with summary review support and unified markdown rendering.
- **Diagrams**: Implemented a new unified diagram engine and custom DSL with SVG rendering and auto-healing parser resilience.
- **Infrastructure**: Transitioned to compile-time embedded database migrations for improved distribution stability.

### Changed

- **UI**: Standardized review visuals, icons, and "Send to PR" list patterns. Centralized all overlays into a modular state system.
- **Domain**: Refactored monolithic domain structures into focused submodules (task, review, feedback).
- **Settings**: Modularized settings view layout and organized configurations by feature.
- **Seed Data**: Switched seed database script to a data-driven approach using external fixtures.

### Fixed

- **Diagrams**: Resolved multi-participant note rendering in D2.
- **UI**: Restored toolbar dropdown icon and checkbox click interactions; improved markdown layout alignment.
- **Project**: Set `lareview` as the default binary for `cargo run`.

## [0.0.15] - 2025-12-30

### Added

- **Export**: Enhanced markdown export with diagrams, improved formatting, selection/options UI, and copy-to-clipboard functionality.
- **Review**: Added feedback deletion capability.
- **D2**: Added async D2 to ASCII rendering.

### Changed

- **Refactor**: Renamed "threads" to "feedback" across the domain and database.
- **UI State**: Moved transient UI state to `UiMemory` for better state management.
- **UI**: Updated `ListItem` and icons for selection support; minor polish and theme fixes.

### Fixed

- **Markdown**: Prevented hangs on large code blocks and improved caching.
- **App**: Improved error handling and robustness across export, prompts, and UI.

## [0.0.14] - 2025-12-27

### Added

- **Open in Editor**: Added editor picker and diff integration for open-in-editor flow.
- **Landing**: Added a new landing page with build config and GitHub Pages deploy workflow.

### Changed

- **Branding**: Refreshed logo assets and screenshots; updated README logo.
- **Release**: Minor tweaks to the release workflow.

### Fixed

- **ACP**: Fixed opencode ACP agent handling.

## [0.0.13] - 2025-12-26

### Added

- **Generate Input**: Added GitHub PR URL/owner#num parsing with `gh` metadata/diff fetch and preview.
- **Agent Settings**: Added per-agent executable overrides, environment variables, and custom ACP agents in Settings.
- **ACP Task Flow**: Added streaming `return_task`/`finalize_review` handling, diff ref validation, and task stats via diff index.
- **Testing**: Added UI harness tests, expanded unit coverage, and new integration tests for review ordering and DB workflows.
- **Tooling**: Added login-shell PATH hydration and git remote discovery helpers.

### Changed

- **Navigation**: Removed the Home view; Generate remains the default entry point.
- **Tool Discovery**: Standardized on login-shell PATH plus per-agent overrides for external tools.
- **macOS App Bundle**: Simplified app packaging to use a single binary name; updated codesign target.
- **Settings UI**: Reworked agent settings layout with cards, modals, and aligned grids.
- **Docs**: Updated architecture/dev guidance and test organization notes.

### Fixed

- **Shell PATH**: Resolved PATH edge cases for CLI tool discovery.
- **UI Details**: Improved pill hover tint and layout alignment.

## [0.0.12] - 2025-12-24

### Added

- **Generate View**: Added a spinning animation to the "Generate" tab icon when a review is being generated.
- **Agent Generation**: Added the ability to abort/cancel agent generation from the UI.
- **Licensing**: Project is now dual-licensed under MIT OR Apache-2.0.

### Changed

- **Navigation**: Simplified and improved the header navigation layout with better centering and more robust width calculations.
- **UI Performance**: Increased the UI repaint frequency (to 16ms) during active generation to ensure smooth animations.
- **Contributing**: Added licensing information to contributing guidelines.

### Fixed

- **UI Consistency**: Ensured consistent text opacity and alignment across header tabs.
- **Cleanup**: Fixed unused variable warnings in the review navigation component.

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
