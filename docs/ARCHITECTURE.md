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

## UI state + reducers (direction)

LaReview is moving toward a reducer-style UI architecture so behavior is deterministic and testable:

- **Action**: user intent or external event (e.g., “Run generation”, “GenMsg received”).
- **Reducer**: pure-ish state transitions (no DB/ACP/IO), returning **Command** values.
- **Command runtime**: executes side effects (DB, ACP, filesystem) and emits new Actions back.

Current state:
- The **Generate flow** is wired through this pattern (`src/ui/app/store/`).
- Review and settings screens now dispatch actions through reducers + commands instead of mutating state directly.

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
- Generate screen (UI): `src/ui/views/generate/` (screen/plan/timeline)
- Review screen (UI): `src/ui/views/review/` (screen/nav/selection/task_detail)
