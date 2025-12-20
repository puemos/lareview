# Architecture

LaReview follows a layered architecture to keep core concepts stable and keep IO/UI concerns at the edges.

## Layers

### `src/domain/` (pure)
- Core types and invariants: entities/value objects like `ReviewTask`, `TaskStatus`, `RiskLevel`.
- No dependencies on `egui`, `rusqlite`, `tokio`, filesystem, or network.
- Prefer putting intrinsic logic on the types themselves (e.g., `RiskLevel::rank()`, `TaskStatus::is_closed()`).

### `src/application/` (use-cases + policies)
- App-specific policies and orchestration that operate on domain types.
- Should not depend on UI frameworks or storage implementations.
- Examples: review ordering/selection policies.

### `src/infra/` (adapters + IO)
- Integrations with the outside world.
- SQLite persistence (`src/infra/db/`), ACP integration (`src/infra/acp/`), and external-format parsing (`src/infra/diff.rs`).
- Keep parsing/normalization of external formats here unless it becomes a first-class domain concept.

### `src/ui/` (presentation)
- `egui`/`eframe` UI.
- `src/ui/components/`: reusable UI widgets (buttons, pills, badges). Avoid business logic here.
- `src/ui/views/`: screens. Keep views thin; lean on `application/` for policies and `infra/` for integrations.

## UI store (reducers + commands)

UI logic now goes through a reducer-style store in `src/ui/app/store/` so state is deterministic and testable:

- **Action** (`action.rs`): user intent or external event (Navigation, Generate, Review, Settings, Async).
- **Reducer** (`reducer.rs`): pure-ish state transitions that mutate `AppState` and emit **Command** values for side effects.
- **Command runtime** (`runtime.rs`): executes side effects (DB, ACP, filesystem), then dispatches follow-up `AsyncAction`s.
- **Dispatch entrypoint** (`mod.rs`): `LaReviewApp::dispatch` runs the reducer, then feeds commands to the runtime.

Flow highlights:
- Generate: `RunRequested` validates diff input, flips `is_generating`, clears the timeline, and emits `StartGeneration {pull_request, diff_text, selected_agent_id}`. Async ACP messages update the timeline; `Done(Ok)` triggers `RefreshReviewData::AfterGeneration` and view switching.
- Review: `RefreshFromDb` emits `RefreshReviewData`, applied in `Async::ReviewDataLoaded` which reselects PR/task invariants and refreshes threads. Status actions clear errors and enqueue DB commands; cleanup removes DONE tasks and refreshes.
- Settings: D2 install/uninstall requests gate on `allow_d2_install`/`is_d2_installing` and emit `RunD2` commands; async output is streamed back.

Add new UI behaviors by introducing an `Action` variant, handling it in `reducer.rs` (with tests there), and emitting a `Command` that the runtime can execute.

## Dependency rules (intent)
- `domain` depends on nothing internal.
- `application` depends on `domain`.
- `infra` depends on `domain` (and may depend on `application` if implementing ports later).
- `ui` depends on `application`, `domain`, and `infra`.

If you’re unsure where something goes:
- **Is it a core concept/invariant?** → `domain`
- **Is it a product policy/use-case?** → `application`
- **Is it IO/parsing/external integration?** → `infra`
- **Is it a widget/layout/rendering?** → `ui`

## Current structure map
- Task generation (ACP): `src/infra/acp/task_generator/` (client/prompt/worker/validation)
- MCP task server (tools + parsing): `src/infra/acp/task_mcp_server/`
- Local persistence (SQLite): `src/infra/db/`
- SQLite repositories: `src/infra/db/repository/` (task/pull_request/note)
- Diff parsing/normalization: `src/infra/diff.rs`
- Review display ordering: `src/application/review/ordering.rs`
- Diff UI component: `src/ui/components/diff/` (model/parse/render)
- App shell + store: `src/ui/app/` (init/header/polling/overlay/update + `store/`)
- Store plumbing (actions/reducer/commands/runtime): `src/ui/app/store/`
- Generate screen (UI): `src/ui/views/generate/` (screen/plan/timeline)
- Review screen (UI): `src/ui/views/review/` (screen/nav/selection/task_detail)
