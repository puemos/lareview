# Architecture

LaReview follows a layered architecture to keep core concepts stable and keep IO/UI concerns at the edges.

## Layers

### `src/domain/` (pure)

- Core types and invariants: entities/value objects like `ReviewTask`, `Review`, `ReviewRun`, `ReviewStatus`, `RiskLevel`, `Feedback`, `Comment`.
- `ReviewSource`: Distinguishes between manual diff pastes and GitHub Pull Requests.
- `Review` and `ReviewRun`: Separation between a review entity and its specific generation attempts.
- `Feedback` and `Comment`: Feedback items associated with tasks or specific lines of code.
- `Plan` and `PlanEntry`: AI-generated roadmap for the review process.
- No dependencies on `tauri`, `rusqlite`, `tokio`, filesystem, or network.
- Prefer putting intrinsic logic on the types themselves (e.g., `RiskLevel::rank()`, `ReviewStatus::is_closed()`).

### `src/application/` (use-cases + policies)

- App-specific policies and orchestration that operate on domain types.
- Should not depend on UI frameworks or storage implementations.
- Examples: review ordering/selection policies, review export logic.

### `src/infra/` (adapters + IO)

- Integrations with the outside world.
- SQLite persistence (`src/infra/db/`), ACP integration (`src/infra/acp/`), and diff parsing (`src/infra/diff/`).
- `src/infra/github/`: GitHub API integration for fetching PRs and syncing comments.
- `src/infra/d2/`: D2 diagramming tool integration for generating architecture diagrams.
- `src/infra/vcs/`: Git operations (local remotes, diff parsing).
- `src/infra/cli/`: CLI commands for `lareview` binary.
- Keep parsing/normalization of external formats here unless it becomes a first-class domain concept.

### `frontend/` (presentation - React + TypeScript)

- React UI built with Vite, Tailwind CSS, and Tauri for native desktop capabilities.
- `src/components/`: reusable UI components.
- `src/hooks/`: custom React hooks for Tauri IPC and state management.
- `src/store/`: Zustand store for frontend state.
- Views include: `Generate`, `Review`, `Repos` (for managing linked repositories), and `Settings`.
- Communicates with Rust backend via Tauri commands.

### `src/commands/` (Tauri IPC bridge)

- Tauri commands that the React frontend invokes.
- Handles serialization/deserialization between frontend and backend.
- Example: `generate_review`, `parse_diff`, `export_review`.

## State Management

### Backend State (`AppState`)

- **Location**: `src/state/mod.rs`.
- **Scope**: Domain state (linked repos, configurations) and database connection.
- **Update Pattern**: Initialized once at app startup, accessed via Tauri State in commands.

### Frontend State (Zustand)

- **Location**: `frontend/src/store/index.ts`.
- **Scope**: UI state, selected review/run/task, feedback list, progress messages.
- **Update Pattern**: Mutated via Zustand actions, persisted to backend via Tauri commands.

## Tauri IPC Flow

1. Frontend calls Tauri command (e.g., `invoke('generate_review', {...})`).
2. Command in `src/commands/mod.rs` executes business logic.
3. Command accesses `AppState` for database operations.
4. Command may spawn async tasks for ACP agent communication.
5. Progress events streamed back via Tauri Channel.
6. Frontend receives events and updates Zustand store.

## Dependency rules (intent)

- `domain` depends on nothing internal.
- `application` depends on `domain`.
- `infra` depends on `domain`.
- `commands` depends on `domain` and `infra`.
- `frontend` depends on `commands` (via Tauri invoke) and manages its own state.

If you're unsure where something goes:

- **Is it a core concept/invariant?** → `domain`
- **Is it a product policy/use-case?** → `application`
- **Is it IO/parsing/external integration?** → `infra`
- **Is it a widget/layout/rendering in the frontend?** → `frontend`
- **Is it bridging frontend to backend?** → `commands`

## Current structure map

- Task generation (ACP): `src/infra/acp/task_generator/` (client/prompt/worker/validation)
- MCP task server: `src/infra/acp/task_mcp_server/`
  - **Tools**: `return_task`, `finalize_review`, `add_comment` (for targeted line feedback), `repo_search`, and `repo_list_files`.
- Local persistence (SQLite): `src/infra/db/`
- SQLite repositories: `src/infra/db/repository/` (task/review/feedback/comment)
- GitHub Integration: `src/infra/vcs/github.rs`
- Diagram Generation (D2): `src/infra/diagram.rs`
- Diff parsing/normalization: `src/infra/diff/`
- Review display ordering: `src/domain/review/ordering.rs`
- Review Export: `src/commands/mod.rs` (markdown export)
- Tauri commands (IPC bridge): `src/commands/mod.rs`
- Frontend (React): `frontend/src/` (components, hooks, store, views)
