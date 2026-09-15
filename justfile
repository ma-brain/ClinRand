# Project command aliases.

# Needed so `cli`'s `*ARGS` reaches the recipe body as real positional
# parameters ($@) instead of one space-joined string — otherwise a quoted
# multi-word argument (e.g. `--operator "Jane Statistician"`) gets re-split
# on whitespace and breaks. See `just --help` / the `positional-arguments`
# setting.
set positional-arguments := true

# Fetch workspace dependencies.
setup:
    cargo fetch

# Run the CLI (for the desktop app, use `just desktop-dev`).
dev:
    cargo run -p clinrand-cli

# Run all workspace tests.
test:
    cargo test --workspace

# Run qc.R PASS/FAIL integration tests (requires R + jsonlite + digest).
qc:
    cargo test -p clinrand-package --test qc_r

# Lint and format-check.
lint:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings

# Build release artifacts.
build:
    cargo build --workspace --release

# Run the CLI wrapper. Uses "$@" (not {{ARGS}}) so quoted multi-word
# arguments survive — see the `positional-arguments` setting above.
cli *ARGS:
    cargo run -p clinrand-cli -- "$@"

# Frontend diagnostics + build, then Tauri host fmt/clippy/check/test.
# Requires Node + Rust + system webview deps (see apps/desktop/README.md).
desktop-check:
    cd apps/desktop && npm ci && npm run check && npm run build
    cd apps/desktop/src-tauri && cargo fmt --all -- --check
    cd apps/desktop/src-tauri && cargo clippy --all-targets -- -D warnings
    cd apps/desktop/src-tauri && cargo check --all-targets
    cd apps/desktop/src-tauri && cargo test

# Run the desktop host tests (command-flow integration + command units).
desktop-test:
    cd apps/desktop/src-tauri && cargo test

# Run the desktop app in development (requires system webview).
desktop-dev:
    cd apps/desktop && npm install && npm run tauri dev

# Confirm the capabilities grant no http/shell/process/updater permission.
desktop-caps-check:
    ! grep -rnE '"(http|shell|process|updater):' apps/desktop/src-tauri/capabilities
