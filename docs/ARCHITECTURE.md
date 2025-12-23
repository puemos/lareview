# Architecture

LaReview follows a layered architecture to keep core concepts stable and keep IO/UI concerns at the edges.

## Layers

### `src/domain/` (pure)
- Core types and invariants: entities/value objects like `ReviewTask`, `Review`, `ReviewRun`, `ReviewStatus`, `RiskLevel`, `Thread`, `Comment`.
- `ReviewSource`: Distinguishes between manual diff pastes and GitHub Pull Requests.
- `Review` and `ReviewRun`: Separation between a review entity and its specific generation attempts.
- `Thread` and `Comment`: Feedback items associated with tasks or specific lines of code.
- `Plan` and `PlanEntry`: AI-generated roadmap for the review process.
- No dependencies on `egui`, `rusqlite`, `tokio`, filesystem, or network.
- Prefer putting intrinsic logic on the types themselves (e.g., `RiskLevel::rank()`, `ReviewStatus::is_closed()`).

### `src/application/` (use-cases + policies)
- App-specific policies and orchestration that operate on domain types.
- Should not depend on UI frameworks or storage implementations.
- Examples: review ordering/selection policies, review export logic.

### `src/infra/` (adapters + IO)
- Integrations with the outside world.
- SQLite persistence (`src/infra/db/`), ACP integration (`src/infra/acp/`), and external-format parsing (`src/infra/diff.rs`).
- `src/infra/github/`: GitHub API integration for fetching PRs and syncing comments.
- `src/infra/d2/`: D2 diagramming tool integration for generating architecture diagrams.
- `src/infra/git/` and `src/infra/brew/`: Local tool integrations for git operations and dependency management.
- Keep parsing/normalization of external formats here unless it becomes a first-class domain concept.

### `src/ui/` (presentation)
- `egui`/`eframe` UI.
- `src/ui/components/`: reusable UI widgets (buttons, pills, badges). Avoid business logic here.
- `src/ui/views/`: screens. Keep views thin; lean on `application/` for policies and `infra/` for integrations.
- Views include: `Generate`, `Review`, `Repos` (for managing linked repositories), and `Settings`.

## UI store (reducers + commands)

UI logic goes through a reducer-style store in `src/ui/app/store/` so state is deterministic and testable:

- **Action** (`action.rs`): user intent or external event (Navigation, Generate, Review, Settings, Async).
- **Reducer** (`reducer.rs`): pure-ish state transitions that mutate `AppState` and emit **Command** values for side effects.
- **Command runtime** (`runtime.rs`): executes side effects (DB, ACP, GitHub, D2, filesystem), then dispatches follow-up `AsyncAction`s.
- **Dispatch entrypoint** (`mod.rs`): `LaReviewApp::dispatch` runs the reducer, then feeds commands to the runtime.

Flow highlights:
- Generate: `RunRequested` validates diff input, flips `is_generating`, clears the timeline, and emits `StartGeneration`. Supports both manual diffs and GitHub PRs.
- Review: Centralized selection of reviews, runs, and tasks. Manages feedback threads and comments.
- Settings: Manages tool requirements (like D2), GitHub authentication status, and extra paths for tool discovery.

Add new UI behaviors by introducing an `Action` variant, handling it in `reducer.rs` (with tests there), and emitting a `Command` that the runtime can execute.

## Dependency rules (intent)
- `domain` depends on nothing internal.
- `application` depends on `domain`.
- `infra` depends on `domain`.
- `ui` depends on `application`, `domain`, and `infra`.

If you’re unsure where something goes:
- **Is it a core concept/invariant?** → `domain`
- **Is it a product policy/use-case?** → `application`
- **Is it IO/parsing/external integration?** → `infra`
- **Is it a widget/layout/rendering?** → `ui`

## Current structure map
- Task generation (ACP): `src/infra/acp/task_generator/` (client/prompt/worker/validation)
- MCP task server: `src/infra/acp/task_mcp_server/`
    - **Tools**: `return_task`, `finalize_review`, `add_comment` (for targeted line feedback), `repo_search`, and `repo_list_files`.
- Local persistence (SQLite): `src/infra/db/`
- SQLite repositories: `src/infra/db/repository/` (task/review/thread/comment)
- GitHub Integration: `src/infra/github.rs`
- Diagram Generation (D2): `src/infra/d2.rs`
- Diff parsing/normalization: `src/infra/diff.rs`
- Review display ordering: `src/application/review/ordering.rs`
- Review Export: `src/application/review/export.rs`
- Diff UI component: `src/ui/components/diff/` (model/parse/render)
- App shell + store: `src/ui/app/` (init/header/polling/overlay/update + `store/`)
- Store plumbing (actions/reducer/commands/runtime): `src/ui/app/store/`
- Generate screen (UI): `src/ui/views/generate/`
- Review screen (UI): `src/ui/views/review/`
- Repos screen (UI): `src/ui/views/repos.rs`
- Settings screen (UI): `src/ui/views/settings.rs`
