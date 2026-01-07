# Contributing to LaReview

Thanks for your interest in improving LaReview! This document outlines the basics to get a clean, reproducible dev setup and how to submit changes.

## Quick start

- Install Rust nightly (edition 2024) with `rustup` and ensure `rustfmt` and `clippy` components are available.
- System deps: `libxkbcommon-dev` and `libxkbcommon-x11-dev` (for Tauri on Linux).
- Clone the repo, then run:
  - `cargo fmt -- --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
- Optional but encouraged: `cargo deny check` (also runs in scheduled CI) to spot advisory/license issues early.
- UI/dev loop: `cargo tauri dev` launches the desktop app with hot reload.
- Local data lives in `~/.local/share/lareview/db.sqlite` (Linux) or `~/Library/Application Support/LaReview/db.sqlite` (macOS); `cargo run --bin reset_db --features dev-tools` clears it, `cargo run --bin seed_db --features dev-tools` populates sample data.

## Pull requests

- Keep PRs small and focused; prefer stacked PRs over large ones.
- Include tests when changing logic; skip only when there’s no reasonable seam.
- Run fmt + clippy + tests before pushing; CI enforces the same commands.
- Describe behavior changes and risks in the PR body; call out breaking changes explicitly.

## Coding conventions

- Edition: 2024, default to stable Rust (see `rust-toolchain.toml`).
- Lints: treat Clippy warnings as errors in CI.
- Avoid `unwrap/expect` outside tests unless failure is impossible; prefer explicit errors surfaced to the UI.
- Keep modules small and cohesive. If a file grows beyond ~400 lines, consider splitting before adding more.

## Test organization

- Unit tests: Located alongside the module they test in `src/` directory (e.g., `src/application/review/tests.rs`).
- Integration tests: Located in the root `tests/` directory (e.g., `tests/database_workflow_integration.rs`).
- End-to-end tests: (Planned) Will be located in `tests/e2e/` for full application workflows.

## Security & networked behavior

- ACP agent invocations and D2 installation are user-triggered; avoid adding implicit network calls.
- Prefer pinned versions for external tools; if you must use `@latest`, document why and how to override.

## License of contributions

By submitting a contribution (code, docs, or tests), you agree that your contribution is licensed under the project’s license:
MIT OR Apache-2.0.

## Reporting issues

- Use GitHub issues with clear repro steps, expected/actual behavior, logs, and platform info.
- Security issues: please follow `SECURITY.md`.
