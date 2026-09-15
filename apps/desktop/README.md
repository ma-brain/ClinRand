# ClinRand desktop application

SvelteKit (static SPA) + Tauri 2 host for ClinRand. Phase 7 is complete: all
§11.1 screens are implemented (config builder, structure preview, generate,
package viewer, unblinded view, validation, about), backed by Tauri commands
that call `clinrand-core` / `clinrand-package`. TypeScript never reimplements
allocation, hashing, canonicalization, or validation.

The Tauri host is locked down per plan §11.2: only the `dialog` and `fs`
plugins are registered, `fs` has no static scope, and there is no
`http`/`shell`/`process`/`updater` capability. See
[`docs/security-posture.md`](../../docs/security-posture.md) for how a reviewer
verifies the no-network posture from the capability files.

## Layout

```text
apps/desktop/
  src/                 SvelteKit frontend (adapter-static, ssr=false, SPA)
  src-tauri/           Tauri 2 Rust host
    capabilities/      Capability lock-down (dialog + scopeless fs only)
    tauri.conf.json    CSP, bundle targets, window config
```

## Cargo workspace boundary (decision)

`src-tauri` is **not** a member of the root `Cargo.toml` workspace. Its
`Cargo.toml` carries its own empty `[workspace]` table, making it a standalone
workspace root that path-depends on `clinrand-core` and `clinrand-package`.

Rationale: this keeps the desktop GUI toolchain (WebKit/WebView, Tauri, plugins)
out of `cargo test --workspace` and the engine CI matrix. The engine crates stay
fast to build and test in isolation, while the desktop app still consumes them
through ordinary path dependencies. Allocation, hashing, canonicalization, and
validation live only in those crates — the desktop host wraps them, never
reimplements them.

## Capability lock-down

Per AGENTS.md §4.5–4.6 and plan §11.2, the capability configuration is
deliberately minimal:

- **No** `http`, `shell`, `process`, or `updater` permission — the application
  is provably incapable of transmitting a randomization list.
- Only the `dialog` and `fs` plugins are registered.
- `fs` has **no static scope** (no allowlist, no `$HOME` wildcard). The dialog
  plugin adds a chosen directory to the fs scope at runtime (`allow_directory`),
  so filesystem access is limited to paths the operator explicitly picks.
- CSP: `default-src 'self'`, no remote fonts/scripts, `connect-src 'self'`. All
  assets are bundled.
- Bundle targets: `dmg`, `app`, `nsis`, `deb`, `appimage` only — no mobile
  targets are configured.

## Develop

```bash
npm install          # from apps/desktop
npm run tauri dev    # run the desktop app (requires Rust + system webview)
```

`cargo check` / `cargo build` / `cargo test` run from
`apps/desktop/src-tauri`. From the repo root, `just desktop-check`,
`just desktop-test`, and `just desktop-dev` wrap these.

## UI smoke flow (config → preview → generate → verify)

This is the manual smoke test that covers the plan §12 "Done when" flow. The
command layer is also exercised without a GUI by the automated integration
test at `src-tauri/tests/command_flow.rs`; the steps below verify the same
sequence through the actual UI.

1. **Config builder** — build a `DEMO`/`TEST`/`EXAMPLE` study (arms + ratios,
   method, block scheme, any stratification, list length, numbering). Live
   validation should clear once the config is valid.
2. **Structure preview** — confirm stratum combinations, per-arm ratio totals,
   block counts/sizes, and total records. It must **never** show a generated
   arm assignment.
3. **Generate** — enter an operator name, pick a destination folder via the OS
   dialog, accept the confirmation that the output contains unblinded material,
   and generate. The screen shows the package path and `list_sha256`. The seed
   is never displayed. Optionally check "Encrypt restricted files" and enter a
   passphrase twice — the result panel then shows `restricted.age` in place of
   the plaintext restricted files (plan §6.6).
4. **Package viewer** — open the generated package, run verify (expect
   checksums + properties PASS — or, for an encrypted package not yet
   decrypted, checksums PASS with a "properties not checked" note), and read
   the inline blinded report. The blinded report never pairs a randomization
   number with an arm.
5. **Unblinded view** *(optional)* — reachable only after an explicit
   confirmation dialog. Opening it appends a timestamped line to
   `access-log.txt` inside the package directory. This access log is a UI-side
   audit convenience, not a security boundary (see
   [`docs/security-posture.md`](../../docs/security-posture.md) §3). For an
   encrypted package, a passphrase prompt appears after the access-log entry
   is written and before any content is decrypted or shown; a correct
   passphrase decrypts the restricted files in place and reveals the report.
6. **Validation** — run each tier and confirm the evidential status is
   labelled: reference = correctness vs. an external source; properties =
   invariants only; regression = consistency only (never correctness).

## Three-OS notes

`npm run tauri dev` / `npm run tauri build` work on macOS, Windows, and Linux.
Prerequisites per platform:

- **macOS** — WebKit ships with the OS; install Xcode command-line tools.
- **Windows** — WebView2 runtime (bundled with recent Windows 10/11; otherwise
  install the Evergreen WebView2 runtime).
- **Linux** — install the WebKitGTK toolchain. On Ubuntu:

  ```bash
  sudo apt-get install -y \
    libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev \
    libayatana-appindicator3-dev libssl-dev libxdo-dev \
    build-essential file wget
  ```

  The `desktop` CI job installs the same packages and runs `cargo check` plus
  the command-flow test on ubuntu. Full GUI end-to-end testing on all three
  OSes is manual (the flow above); CI does not drive the WebView.
