# Development

## Prerequisites

- Rust 1.99.1: see `rust-toolchain.toml` (includes `rustfmt` and `clippy`; MSRV 1.91)
- Node.js 24.22.2 and pnpm 11.28.9
- Linux system deps: `libxkbcommon-dev`, `libxkbcommon-x11-dev`

## Common commands

- Run the app: `cargo run`
- Format: `cargo fmt`
- Lint: `cargo clippy --all-targets --all-features -- -D warnings`
- Test: `cargo test --all-targets`
- Supply chain: `cargo deny check`
- Frontend: `cd frontend && pnpm install --frozen-lockfile && pnpm lint && pnpm test && pnpm build`
- Landing page: `cd landing && pnpm install --frozen-lockfile && pnpm build`

## Dependency security

- Run `pnpm audit` in both `frontend/` and `landing/`; CI checks both lockfiles on pull requests and weekly.
- Run `cargo deny --locked check` for Rust advisories, licenses, and dependency sources.
- `frontend/pnpm-workspace.yaml` overrides Monaco's DOMPurify, Mermaid's KaTeX, and Tailwind Typography's selector parser with patched releases. Remove these overrides once upstream requirements include the fixes.
- Mermaid stays on the latest 11.x release to preserve support for existing desktop webviews; Mermaid 12 requires Safari 17.4 or newer.
- `deny.toml` documents upstream exceptions for Tauri's GTK3 stack (`proc-macro-error`, `glib`) and its Linux single-instance plugin (`zbus_names`, `zvariant`, `zvariant_derive`). These remain maintenance/soundness warnings with no compatible upstream fix.
- Keep the Rust 1.91 MSRV when updating dependencies. `parking_lot` 0.13 requires Rust 1.95, so this project uses the maintained 0.12 series.

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
