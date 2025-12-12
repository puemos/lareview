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

