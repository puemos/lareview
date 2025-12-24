# GEMINI.md

This file provides a comprehensive overview of the `lareview` project, intended to be used as instructional context for Gemini.

## Project Overview

`lareview` is a desktop application for code review, built with Rust. It provides a graphical user interface to review code changes (diffs), generate review tasks, and add notes. The application appears to leverage an external AI agent for parts of the review process, communicating via the Agent Client Protocol (ACP).

### Key Technologies

*   **Language:** [Rust](https://www.rust-lang.org/) (2024 Edition)
*   **GUI Framework:** [`egui`](https://github.com/emilk/egui) and [`eframe`](https://github.com/emilk/egui/tree/master/crates/eframe) for immediate mode GUI.
*   **Asynchronous Runtime:** [`tokio`](https://tokio.rs/) for managing asynchronous operations.
*   **Database:** [`rusqlite`](https://github.com/rusqlite/rusqlite) for local data storage (SQLite).
*   **Diff/Patch Handling:** [`unidiff`](https://crates.io/crates/unidiff) and [`similar`](https://crates.io/crates/similar) for processing code differences.
*   **Templating:** [`handlebars`](https://crates.io/crates/handlebars) for text templating, likely for generating prompts or reports.
*   **Agent Communication:** `pmcp` and `agent-client-protocol` for interacting with an AI agent.

### Architecture

The project follows a modular structure:

*   `src/main.rs`: The application entry point. It initializes the `tokio` runtime and the `eframe` GUI application.
*   `src/ui/app.rs`: Contains the core application logic and state management (`LaReviewApp`, `AppState`). It defines the main UI structure and handles events.
*   `src/ui/views/`: Implements the different screens of the application, such as the `generate_view` and `review_view`. The `review_view` now includes a tree-based navigation system.
*   `src/data/`: Manages data persistence. It includes a `db.rs` for SQLite connection handling and repositories (`TaskRepository`, `NoteRepository`, etc.) for data access.
*   `src/domain/`: Defines the core data structures (structs) used throughout the application, like `PullRequest`, `ReviewTask`, and `Note`. The `ReviewTask` struct now includes a `sub_flow` field.
*   `src/acp/`: Contains logic for the Agent Client Protocol, suggesting communication with an external agent for tasks like generating reviews.
*   `src/prompts/`: Includes `handlebars` templates, likely used to generate prompts for the AI agent.

## UI Views

LaReview has two main views:

*   **GENERATE:** This view allows the user to paste a diff, select an AI agent, and generate a review plan.
*   **REVIEW:** This view displays the review plan in a tree-based navigation system. The user can navigate through the plan, view task details, and add notes.

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
cargo test --verbose
```

### Building the Application

To compile the application:
```bash
cargo build
```

### Running the Application

To build and run the application in development mode:
```bash
cargo run
```

### Database Management Scripts

The project includes utility scripts for database management:

1. **Seed Database** - Populate the database with sample data:
```bash
cargo run --bin seed_db
```

2. **Reset Database** - Clear all data from the database:
```bash
cargo run --bin reset_db
```

## Development Conventions

*   **Formatting:** The project uses `rustfmt` for consistent code formatting.
*   **Linting:** `clippy` is used with a strict warning policy (`-D warnings`), meaning all warnings are treated as errors in the CI pipeline.
*   **Testing:** Unit and integration tests are run with `cargo test`.
*   **Dependencies:** The project uses a specific nightly toolchain (`nightly-2025-12-06`), as defined in `rust-toolchain.toml`.
