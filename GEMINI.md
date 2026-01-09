# GEMINI.md

This file provides a comprehensive overview of the `lareview` project, intended to be used as instructional context for Gemini.

## Project Overview

**LaReview** is a desktop code review companion that generates intent-driven review plans with AI. Built with Rust and Tauri, it provides a graphical user interface to review code changes (diffs), generate review tasks, and add notes. The application uses an external AI agent for parts of the review process, communicating via the Agent Client Protocol (ACP).

### Key Technologies

#### Backend (Rust)

| Technology                                                                | Purpose                 |
| ------------------------------------------------------------------------- | ----------------------- |
| [Rust](https://www.rust-lang.org/) (2024 Edition)                         | Core language           |
| [Tauri](https://tauri.app/) v2                                            | Desktop app shell & IPC |
| [`tokio`](https://tokio.rs/)                                              | Asynchronous runtime    |
| [`rusqlite`](https://github.com/rusqlite/rusqlite)                        | Local SQLite database   |
| [`handlebars`](https://crates.io/crates/handlebars)                       | Prompt templating       |
| [`agent-client-protocol`](https://crates.io/crates/agent-client-protocol) | AI agent communication  |
| [`pmcp`](https://crates.io/crates/pmcp)                                   | MCP client support      |
| [`syntect`](https://crates.io/crates/syntect)                             | Syntax highlighting     |

#### Frontend (React)

| Technology                                                  | Purpose                 |
| ----------------------------------------------------------- | ----------------------- |
| [React](https://react.dev/) v19                             | UI framework            |
| [TypeScript](https://www.typescriptlang.org/)               | Type-safe JavaScript    |
| [Tailwind CSS](https://tailwindcss.com/) v4                 | Styling                 |
| [Vite](https://vitejs.dev/) v7                              | Build tool              |
| [Zustand](https://zustand.docs.pmnd.rs/)                    | State management        |
| [Monaco Editor](https://microsoft.github.io/monaco-editor/) | Code diff viewer        |
| [TanStack Query](https://tanstack.com/query)                | Data fetching & caching |
| [Phosphor Icons](https://phosphoricons.com/)                | Icon library            |
| [Vitest](https://vitest.dev/)                               | Testing framework       |

---

## Architecture

The project follows a modular, layered architecture:

- **Domain layer** (`src/domain/`): Pure business logic, no external dependencies
- **Infrastructure layer** (`src/infra/`): External integrations (DB, VCS, ACP)
- **Commands** (`src/commands/`): Thin wrappers exposing functionality to frontend
- **Components** (`frontend/src/components/`): Organized by feature/view

---

## UI Views

LaReview has a Tauri-based desktop shell with three main views:

| View         | Description                                                                                      |
| ------------ | ------------------------------------------------------------------------------------------------ |
| **Generate** | Paste a diff (or fetch from GitHub PR), select an AI agent, and generate a review plan           |
| **Review**   | Tree-based navigation through the review plan; view task details, code changes, and add feedback |
| **Settings** | Configure AI agents, editor preferences, and application settings                                |

---

## Building and Running

### Prerequisites

- Rust nightly toolchain (see `rust-toolchain.toml`)
- Node.js & pnpm
- Tauri CLI: `cargo install tauri-cli`

### Development Commands

```bash
# Run the full application (backend + frontend)
cargo tauri dev

# Or run frontend dev server separately
cd frontend && pnpm dev
```

### Build Commands

```bash
# Build Rust backend
cargo build

# Build frontend
cd frontend && pnpm build

# Build production release
cargo tauri build
```

### Code Quality

```bash
# Check formatting
cargo fmt -- --check

# Run linter (warnings = errors)
cargo clippy --all-targets --all-features -- -D warnings

# Run Rust tests
cargo test

# Run frontend linter
cd frontend && pnpm lint

# Run frontend tests
cd frontend && pnpm test
```

---

## Development Conventions

### Formatting & Linting

- **Rust:** Uses `rustfmt` for formatting and `clippy` with strict policy (`-D warnings`)
- **Frontend:** Uses ESLint + Prettier with Tailwind plugin
- **Toolchain:** Nightly Rust as defined in `rust-toolchain.toml`

### Git Workflow

1. **Always** run `cargo fmt` and `cargo clippy` before committing
2. Use `git rebase` instead of `git merge` to keep history clean
3. Follow [Conventional Commits](https://www.conventionalcommits.org/) format: `action(scope): subject`
   - Examples: `feat(review): add feedback threading`, `fix(diff): handle binary files`
4. Write concise, descriptive commit messages that summarize all changes
5. Do not commit or push asking for review or approval

### Code Organization

- **Domain layer** (`src/domain/`): Pure business logic, no external dependencies
- **Infrastructure layer** (`src/infra/`): External integrations (DB, VCS, ACP)
- **Commands** (`src/commands/`): Thin wrappers exposing functionality to frontend
- **Components** (`frontend/src/components/`): Organized by feature/view

---

## New Release Process

1. **Preparation:** Ensure all tests pass and code is formatted.
   - **Backend:** `cargo fmt && cargo clippy --all-targets --all-features -- -D warnings && cargo test`
   - **Frontend:** `cd frontend && pnpm lint && pnpm test && pnpm build`
2. **Version Bump:** Update version in:
   - `Cargo.toml`
   - `tauri.conf.json`
   - `Cargo.lock` (run `cargo build` to sync)
3. **Changelog:** Update `CHANGELOG.md` with new version and release notes.
4. **Documentation:** Update `README.md` if needed.
5. **Commit & Tag:**
   ```bash
   git add -A
   git commit -m "chore(release): v0.0.X"
   # Use annotated tag with changelog summary for better release visibility
   git tag -a v0.0.X -m "v0.0.X entry from CHANGELOG.md"
   git push && git push --tags
   ```
6. **Publish:** GitHub CI will handle the rest (builds, releases, Homebrew tap).

---

## Key Data Structures

### Domain Models

| Model        | File                 | Description                                                          |
| ------------ | -------------------- | -------------------------------------------------------------------- |
| `Review`     | `domain/review.rs`   | A review session with metadata and state                             |
| `ReviewTask` | `domain/task.rs`     | Individual review item with severity, description, and code location |
| `Feedback`   | `domain/feedback.rs` | User notes and comments on code                                      |

### Frontend State

| Store            | Purpose                                     |
| ---------------- | ------------------------------------------- |
| `useReviewStore` | Current review session and navigation state |
| React Query      | Server state management and caching         |

---

## External Integrations

### Agent Client Protocol (ACP)

The application communicates with AI agents via ACP for:

- Generating review plans from diffs
- Analyzing code changes for potential issues
- Suggesting review focus areas

### GitHub Integration

- Fetch diffs from GitHub Pull Requests
- Push review feedback as PR comments
- Deep linking support via `lareview://` protocol

---

## Useful Links

- **Repository:** https://github.com/puemos/lareview
- **Landing Page:** https://lareview.dev (source in `landing/`)
- **Changelog:** See `CHANGELOG.md`
- **Contributing:** See `CONTRIBUTING.md`

---

## Agent Guidelines

This section provides guidance for AI coding agents working on this repository.

### Efficiency Tips

- **Text replacement:** Use `sed` for simple, targeted text replacements across files
- **File searching:** Use `rg` (ripgrep) for fast, regex-capable code search
- **Prefer small edits:** Make surgical edits rather than rewriting entire files
- **Batch related changes:** Group related modifications into a single logical commit

### Understanding the Codebase

1. **Start with types:** When exploring a feature, begin with `src/domain/` to understand the core data structures
2. **Follow the IPC boundary:** Commands in `src/commands/` bridge frontend and backend—trace from there
3. **Check the templates:** Prompt logic lives in `*.hbs` files in `src/`
4. **Frontend follows backend:** Frontend types in `frontend/src/types/` mirror Rust structs

### Common Patterns

#### Adding a New Tauri Command

1. Define the function in `src/commands/mod.rs`
2. Register it in `main.rs` via `.invoke_handler()`
3. Call from frontend using `@tauri-apps/api/core`'s `invoke()`

#### Adding a New React Component

1. Create a new directory under `frontend/src/components/`
2. Export from an `index.tsx` file
3. Use existing hooks from `frontend/src/hooks/` for data fetching
4. Follow existing Tailwind patterns from sibling components

#### Database Migrations

1. Add SQL file to `migrations/`
2. Migrations run automatically on app start

### Testing Workflow

```bash
# Quick validation before committing
cargo fmt && cargo clippy --all-targets --all-features -- -D warnings && cargo test

# Frontend validation
cd frontend && pnpm lint && pnpm test && pnpm build
```

### Things to Avoid

- **Don't modify `Cargo.lock` manually**—let Cargo manage it
- **Don't inline Tailwind classes excessively**—extract to components if repeated
- **Don't add new dependencies without justification**—prefer existing crates
- **Don't skip error handling**—use `anyhow::Result` or proper `thiserror` types

### File Naming Conventions

| Location         | Convention       | Example          |
| ---------------- | ---------------- | ---------------- |
| Rust modules     | `snake_case.rs`  | `review_task.rs` |
| React components | `PascalCase.tsx` | `TaskDetail.tsx` |
| React hooks      | `camelCase.ts`   | `useFeedback.ts` |
| Utility files    | `camelCase.ts`   | `formatDate.ts`  |

### Quick Reference Commands

```bash
# Find all Tauri commands
rg "#\[tauri::command\]" src/

# Find React component usage
rg "<ComponentName" frontend/src/

# Find all TODO/FIXME comments
rg "TODO|FIXME" --type rust --type ts

# Check what changed in a file
git log -p --follow -n 5 -- path/to/file
```
