# GEMINI.md

This file provides a comprehensive overview of the `lareview` project, intended to be used as instructional context for Gemini.

## Project Overview

`lareview` is a desktop application for code review, built with Rust. It provides a graphical user interface to review code changes (diffs), generate review tasks, and add notes. The application uses an external AI agent for parts of the review process, communicating via the Agent Client Protocol (ACP).

### Key Technologies

- **Language:** [Rust](https://www.rust-lang.org/) (2024 Edition)
- **GUI Framework:** [Tauri](https://tauri.app/) for desktop app shell with [React](https://react.dev/) + [Tailwind CSS](https://tailwindcss.com/) frontend
- **Asynchronous Runtime:** [`tokio`](https://tokio.rs/) for managing asynchronous operations.
- **Database:** [`rusqlite`](https://github.com/rusqlite/rusqlite) for local data storage (SQLite).
- **Templating:** [`handlebars`](https://crates.io/crates/handlebars) for text templating, likely for generating prompts or reports.
- **Agent Communication:** `agent-client-protocol` for interacting with an AI agent.
- **Diagramming:** [D2](https://d2lang.com/) for architecture diagrams.

### Architecture

The project follows a modular structure:

- `src/main.rs`: The application entry point for the Tauri backend.
- `src/commands/mod.rs`: Tauri commands (Rust) that the frontend calls via IPC.
- `src/ui/` (deprecated): Old egui UI code being phased out.
- `frontend/`: New React + TypeScript frontend built with Vite.
- `src/domain core data structures (/`: Defines thestructs) used throughout the application, like `ReviewTask`, `Review`, `Feedback`.
- `src/infra/acp/`: Contains logic for the Agent Client Protocol, suggesting communication with an external agent for tasks like generating reviews.
- `src/infra/db/`: Database layer with SQLite persistence.
- `src/prompts/`: Includes `handlebars` templates, likely used to generate prompts for the AI agent.

## UI Views

LaReview has a Tauri-based desktop shell with two main views:

- **GENERATE:** This view allows the user to paste a diff, select an AI agent, and generate a review plan.
- **REVIEW:** This view displays the review plan in a tree-based navigation system. The user can navigate through the plan, view task details, and add notes.

## Building and Running

The following commands are used for common development tasks, as inferred from the `ci.yml` workflow.

### Check Formatting

To check if the code is formatted according to project standards:

```bash
cargo fmt -- --check
```

### Git

1. Never commit without running `cargo fmt` and `cargo clippy`.
2. Use `git rebase` instead of `git merge` to keep the commit history clean.
3. Do not commit or push asking for review or approval.
4. Commit structure should follow the conventional commit format `action(scope): subject`.
5. Commit messages should be concise and descriptive and include all stuff done based on the diff.

### Linting

To run the clippy linter and check for warnings:

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

### Running Tests

To execute the test suite:

```bash
cargo test
```

### Building the Application

To compile the Rust backend:

```bash
cargo build
```

To build the frontend:

```bash
cd frontend && pnpm build
```

### Running the Application

To build and run the application in development mode:

```bash
cargo tauri dev
```

Or run frontend dev server separately:

```bash
cd frontend && pnpm dev
```

### Development Conventions

- **Formatting:** The project uses `rustfmt` for consistent code formatting.
- **Linting:** `clippy` is used with a strict warning policy (`-D warnings`), meaning all warnings are treated as errors in the CI pipeline.
- **Testing:** Unit and integration tests are run with `cargo test`.
- **Dependencies:** The project uses a specific nightly toolchain as defined in `rust-toolchain.toml`.

## New Release Process

- **Release Preparation:** Ensure all tests pass and the code is formatted correctly.
- **Version Bump:** Update the version number in `Cargo.toml` and `Cargo.lock`.
- **Documentation:** Update the README and other documentation files.
- **Commit and Tag:** Commit the changes and tag the release.
- **Publish:** Push and push the tags. GitHub CI will do the rest.
