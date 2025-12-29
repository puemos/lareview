# Development

## Prerequisites

- Rust toolchain: see `rust-toolchain.toml` (includes `rustfmt` and `clippy`)
- Linux system deps: `libxkbcommon-dev`, `libxkbcommon-x11-dev`

## Common commands

- Run the app: `cargo run`
- Reset/seed sample data: `cargo run --bin reset_db`, `cargo run --bin seed_db`
- Format: `cargo fmt`
- Lint: `cargo clippy --all-targets --all-features -- -D warnings`
- Test: `cargo test`
- Supply chain: `cargo deny check`

## Useful environment variables

- `LAREVIEW_DB_PATH`: override SQLite path (helpful for tests/dev sandboxes)
- `ACP_DEBUG`: enable ACP debug logging (prints ACP session updates/tool calls)
- `LAREVIEW_CONFIG_PATH`: override the default configuration file path.
- `LAREVIEW_DATA_HOME`: override the default data directory.

## Working with the UI reducer store

- **Global State**: Managed via `AppState` (`src/ui/app/state.rs`) and mutated through actions (`src/ui/app/store/action.rs`). Dispatch actions from views for anything that affects the domain, requires persistence, or triggers async side-effects.
- **Transient State**: Use `UiMemory` (`src/ui/app/ui_memory.rs`) for purely visual concerns like text drafts, toggle states that don't need persistence, or layout dimensions. Components can mutate this directly via `with_ui_memory_mut`.
- **Side Effects**: Reducers should stay side-effect free and return `Command`s; implement side effects in `store/runtime.rs`, then dispatch `AsyncAction` results back through the reducer.
- **Testing**: Add tests in `src/ui/app/store/reducer/` for new actions/commands. Use `src/ui/app/tests/` for full UI harness integration tests.
- **Invariants**: Review data refreshes should go through `ReviewAction::RefreshFromDb` so selection + thread loading invariants remain centralized in the reducer.

## Tool Discovery and Requirements

- LaReview depends on external tools like `d2` for diagram generation and `gh` for GitHub integration.
- Tool discovery uses the process PATH (hydrated from the login shell when running outside a terminal on macOS/Linux) and per-agent overrides in the Settings view.
- Requirements can be checked and optionally installed (for `d2`) directly from the app.
