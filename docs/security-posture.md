# Security posture

This document states the network and filesystem posture of the ClinRand
desktop application and describes how a reviewer can verify it directly from
the source tree, without trusting this prose.

The regulatory claim is simple:

> **The ClinRand desktop application has no network capability compiled in.
> It is provably incapable of transmitting a randomization list.**

The seed is equivalent to the list (AGENTS.md §3, §4.9). A tool that cannot
open a socket cannot exfiltrate either. The verification steps below let an
inspector confirm that claim from the capability configuration rather than
from testimony.

See also: AGENTS.md §4 (hard prohibitions 4–6: no network capability, no
mobile targets, no weakening of the capability configuration) and
`docs/plans/clinrand-implementation-plan.md` §11.2 (Tauri capability
lock-down).

---

## 1. What is granted

The application registers exactly two Tauri plugins — `dialog` and `fs` — and
grants a deliberately minimal capability set. Everything a reviewer needs to
confirm this lives in three files:

| File | What it proves |
|---|---|
| `apps/desktop/src-tauri/capabilities/default.json` | The only permissions granted to the main window. |
| `apps/desktop/src-tauri/tauri.conf.json` | CSP, bundle targets, no remote origins. |
| `apps/desktop/src-tauri/Cargo.toml` | Which plugins are even compiled in. |

---

## 2. How a reviewer verifies no network capability

### 2.1 Capability file

Open `apps/desktop/src-tauri/capabilities/default.json`. The `permissions`
array contains only:

- `core:default`
- `dialog:default`
- `fs:allow-read-file`, `fs:allow-read-text-file`, `fs:allow-write-file`,
  `fs:allow-write-text-file`, `fs:allow-read-dir`, `fs:allow-mkdir`,
  `fs:allow-exists`

Confirm the **absence** of any `http:`, `shell:`, `process:`, or `updater:`
permission identifier:

```bash
# From the repository root. Expect no matches.
rg -n '"(http|shell|process|updater):' apps/desktop/src-tauri/capabilities
```

The pattern targets permission identifiers (`"http:..."`), not the literal
substring `http`, so it does not flag the `$schema` URL or the `devUrl` in
`tauri.conf.json` — neither of which is a granted capability. CI runs this
same grep as a gate (see `.github/workflows/ci.yml`).

### 2.2 No `fs` static scope

The `fs` permissions above carry **no scope entry** — no allowlist, no
`$HOME` wildcard. Filesystem access is limited to paths the operator picks in
an OS dialog; the `dialog` plugin adds a chosen directory to the `fs` scope at
runtime. Confirm there is no static `scope`/`allow` path list in the
capability file.

### 2.3 Compiled plugins

Open `apps/desktop/src-tauri/Cargo.toml`. The only Tauri plugin dependencies
are `tauri-plugin-dialog` and `tauri-plugin-fs`. There is no
`tauri-plugin-http`, `tauri-plugin-shell`, `tauri-plugin-process`, or
`tauri-plugin-updater`. A capability cannot be granted for a plugin that is
not compiled in, and `apps/desktop/src-tauri/src/lib.rs` registers only those
two plugins.

```bash
rg -n 'tauri-plugin-(http|shell|process|updater)' apps/desktop/src-tauri
# Expect no matches.
```

### 2.4 Content Security Policy

Open `apps/desktop/src-tauri/tauri.conf.json` and read `app.security.csp`:

```
default-src 'self'; img-src 'self' asset: data:; style-src 'self' 'unsafe-inline';
font-src 'self'; script-src 'self'; connect-src 'self'; object-src 'none';
base-uri 'self'; form-action 'self'; frame-src 'self'
```

`connect-src 'self'` forbids the WebView from opening `fetch`/XHR/WebSocket
connections to any remote origin. `script-src 'self'` and `font-src 'self'`
forbid remote scripts and fonts; all assets are bundled. There is no remote
`devUrl` in a production build — `frontendDist` points at the locally built
SPA.

### 2.5 Bundle targets

`bundle.targets` in `tauri.conf.json` is `["dmg", "app", "nsis", "deb",
"appimage"]` — desktop installers only. No mobile targets are configured, and
none may be added (AGENTS.md §4, prohibition 5).

### 2.6 Built bundle (optional, strongest evidence)

Plan §11.2 anticipates grepping the built bundle. When a bundle is produced
(`npm run tauri build` from `apps/desktop`), the embedded capability manifest
carries the same permission set; grepping the generated
`apps/desktop/src-tauri/gen/schemas/` and the bundled ACL for the same
identifiers as §2.1 confirms the shipped artifact matches the source.

---

## 3. Scope of this posture

- **No allocation, hashing, or canonicalization runs in TypeScript.** The
  frontend only invokes Tauri commands that call `clinrand-core` /
  `clinrand-package` (AGENTS.md §5). This keeps the trust boundary in audited
  Rust, not in the WebView.
- **The seed is never returned to the frontend, logged, or placed in any
  error string** (AGENTS.md §4.9). The `generate_package` command draws the
  seed, writes it only to `manifest.unblinded.json` on disk, and returns the
  package path, `list_sha256`, and record count — never the seed.
- **The unblinded access log is a UI-side control, not a security boundary.**
  Opening the unblinded view appends a timestamped line to `access-log.txt`
  inside the package directory. It is an audit convenience for the operator; a
  determined operator with filesystem access is out of scope for this posture,
  which concerns network exfiltration only. Command-flow safety sequencing
  (validate → preview → generate → verify) is exercised by the automated Rust
  integration test at `apps/desktop/src-tauri/tests/command_flow.rs`; the
  access-log gate itself lives in the frontend route and is verified by manual
  UI smoke (see `apps/desktop/README.md`).

---

## 3a. Encryption at rest (Phase 8)

`generate_package` accepts an optional `passphrase`; when present, the
restricted files are written encrypted into `restricted.age` instead of as
plaintext (plan §6.6, `docs/decisions/0007-restricted-container-format.md`).
This is entirely local: Argon2id key derivation and XChaCha20-Poly1305
encryption run in the same Rust process, using no network capability and no
plugin beyond `dialog`/`fs`. It does not change the posture in §1–§3 — there
is still no `http`, `shell`, `process`, or `updater` plugin, and the
passphrase follows the same rule as the seed: never returned to the
frontend, logged, or included in any error string.

`decrypt_package` writes the 5 restricted files back to disk as plaintext,
in place, as an explicit operator action — the same trust boundary as an
unencrypted package from that point on. It is not a security boundary
against a user with filesystem access to their own machine, the same caveat
already stated for the unblinded access log in §3.

---

## 4. What would violate this posture

Any of the following is a regression that must be rejected in review:

- Adding a `http:`, `shell:`, `process:`, or `updater:` permission to a
  capability file.
- Adding a `tauri-plugin-http`/`-shell`/`-process`/`-updater` dependency.
- Adding a static `fs` scope (allowlist or `$HOME` wildcard).
- Loosening the CSP `connect-src`, `script-src`, or `font-src` beyond
  `'self'`.
- Adding telemetry, crash reporting, or an auto-updater by any other means.
- Configuring a mobile bundle target.
