# ClinRand desktop application

SvelteKit (static SPA) + Tauri 2 host for ClinRand. This is the Phase 7
scaffold: a single placeholder home page and a locked-down Tauri shell. Real
screens and Tauri commands are added in later Phase 7 tasks.

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

`cargo check` / `cargo build` run from `apps/desktop/src-tauri`.
