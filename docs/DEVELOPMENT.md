# Development

## Prerequisites

- Rust 1.97.1: see `rust-toolchain.toml` (includes `rustfmt` and `clippy`; MSRV 1.91)
- Node.js 24.18.0 and pnpm 11.20.0
- Linux system deps: `libxkbcommon-dev`, `libxkbcommon-x11-dev`

## Common commands

- Run the app: `cargo run`
- Format: `cargo fmt`
- Lint: `cargo clippy --all-targets --all-features -- -D warnings`
- Test: `cargo test --all-targets`
- Supply chain: `cargo deny check`
- Frontend: `cd frontend && pnpm install --frozen-lockfile && pnpm lint && pnpm test && pnpm build`
- Landing page: `cd landing && pnpm install --frozen-lockfile && pnpm build`

## Useful environment variables

- `LAREVIEW_DB_PATH`: override SQLite path (helpful for tests/dev sandboxes)
- `ACP_DEBUG`: enable ACP debug logging (prints ACP session updates/tool calls)
- `LAREVIEW_CONFIG_PATH`: override the default configuration file path.
- `LAREVIEW_DATA_HOME`: override the default data directory.

## Working with the frontend store

- **Global state**: Zustand store in `frontend/src/store/index.ts`.
- **Server state**: TanStack Query hooks in `frontend/src/hooks/` call Tauri commands.
- **Components**: React components live in `frontend/src/components/`.
- **Testing**: Frontend tests run with Vitest and Testing Library.

## Tool Discovery and Requirements

- LaReview depends on external tools like `d2` for diagram generation and `gh` for GitHub integration.
- Tool discovery uses the process PATH (hydrated from the login shell when running outside a terminal on macOS/Linux) and per-agent overrides in the Settings view.
- Requirements can be checked and optionally installed (for `d2`) directly from the app.
