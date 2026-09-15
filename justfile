# Project command aliases.

# Fetch workspace dependencies.
setup:
    cargo fetch

# Run the CLI placeholder (desktop app arrives in Phase 7).
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

# Run the CLI wrapper.
cli *ARGS:
    cargo run -p clinrand-cli -- {{ARGS}}
