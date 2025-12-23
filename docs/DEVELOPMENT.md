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
- `LAREVIEW_EXTRA_PATH`: additional directories to add to the `PATH` for tool discovery (e.g., if `d2` is installed in a non-standard location).
- `LAREVIEW_CONFIG_PATH`: override the default configuration file path.
- `LAREVIEW_DATA_HOME`: override the default data directory.

## Working with the UI reducer store
- UI state lives in `src/ui/app/state.rs` and is mutated through reducer actions (`src/ui/app/store/action.rs` + `reducer.rs`). Dispatch actions from views instead of mutating state directly.
- Reducers should stay side-effect free and return `Command`s; implement side effects in `store/runtime.rs`, then dispatch `AsyncAction` results back through the reducer.
- Add tests in `store/reducer.rs` for new actions/commands to lock in selection rules, error handling, and view switching.
- Review data refreshes should go through `ReviewAction::RefreshFromDb` so selection + thread loading invariants remain centralized in the reducer.

## Tool Discovery and Requirements
- LaReview depends on external tools like `d2` for diagram generation and `gh` for GitHub integration.
- Tool discovery can be managed via the Settings view or by setting `LAREVIEW_EXTRA_PATH`.
- Requirements can be checked and optionally installed (for `d2`) directly from the app.
