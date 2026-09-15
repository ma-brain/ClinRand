# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Critical reading

**Read `AGENTS.md` completely before making any change.** It specifies hard prohibitions, determinism rules, and crate boundaries that override any other guidance. The most important rules:

1. **Determinism contract**: `(config, seed)` pairs must produce byte-identical output forever until `ALGO_VERSION` is deliberately incremented. See `AGENTS.md §3`.
2. **Seed is secret**: Never log, display, test-fixture, or commit it. See `AGENTS.md §4.9`.
3. **Validation tiers have different evidential weight**: reference (correctness), properties (invariants), regression (no silent changes). Never move cases between tiers. See `AGENTS.md §7`.
4. **Hard prohibitions**: §4 of `AGENTS.md` lists 13 things that require explicit written approval from the project owner.

Also read:
- `docs/plans/clinrand-implementation-plan.md` — specification for the current phase
- `docs/determinism.md` — the algorithm contract (ChaCha20, uniform_below, Fisher–Yates, stream order)
- `validation/README.md` — what each validation tier proves and does not prove
- `apps/desktop/README.md` — desktop architecture and capability lock-down

## Project overview

**ClinRand** generates randomization lists for clinical trials and the audit evidence needed for QC and approval. The tool is:

- **Deterministic**: same config + seed = same output, forever (unless `ALGO_VERSION` changes)
- **Offline**: no network capability compiled in; verified by capability files
- **Regulatory**: used by an unblinded statistician who must later prove the approved list matches what the tool produced
- **Three-part**: Rust engine (`clinrand-core`), package I/O (`clinrand-package`), CLI and desktop UI on top

See `README.md` for scope (v1: simple randomization, permuted blocks, stratified blocks) and what's out of scope (minimization, runtime allocation, kit lists — those belong to **myIWRS**).

## Crate boundaries

Keep allocation, hashing, and validation logic in the right crates:

| Crate | Owns | Must not |
|-------|------|----------|
| `clinrand-core` | Allocation (`generate`), uniform RNG, permutation, stream semantics | I/O, clock, env vars, `getrandom` (except once at run start), floating point |
| `clinrand-package` | Package format (CSV, manifests, reports), hashing, canonicalization | Any allocation logic |
| `clinrand-cli` | CLI argument parsing, I/O orchestration | Allocation or hashing logic |
| `apps/desktop/src` (TypeScript) | UI, layout, dialog integration | Allocation logic; call Tauri commands instead |

`clinrand-core::generate` is a pure function of `(config, seed)` only — no paths, no clock. Enforce this: no `use std::fs`, `std::time`, or `std::env` in that crate. If you need I/O to make something work, surface the problem instead of adding imports.

## Development commands

Use `just` (installed locally, see `justfile` for task names) or run cargo directly.

### Workspace (all three crates)

```bash
just setup          # cargo fetch
just test           # cargo test --workspace
just lint           # rustfmt --all --check + clippy -D warnings
just build          # cargo build --workspace --release
```

### CLI only

```bash
just cli version                                      # print version
just cli generate --config examples/simple.json --out /tmp/out --operator "Jane Statistician"
```

### Desktop (Tauri + SvelteKit)

Prerequisites: Rust stable, Node 20+, system WebView (WebKit on macOS, WebView2 on Windows, WebKitGTK on Linux).

```bash
cd apps/desktop
npm install
npm run tauri dev                                   # live-reload dev server
```

From repo root (convenience wrappers):

```bash
just desktop-check          # frontend build + Tauri fmt/clippy/check/test
just desktop-test           # command-flow integration test
just desktop-dev            # npm install + tauri dev
just desktop-caps-check     # verify no http/shell/process/updater capability
```

See `apps/desktop/README.md` for manual smoke-test flow (config → preview → generate → verify).

### Validation (three tiers)

Reference tier (external sources): `validation/reference/` contains ChaCha20 RFC 8439 vectors and hand-worked examples for Fisher–Yates and uniform_below.

Properties tier (invariants): `validation/properties/` holds property tests over arbitrary valid configs (minimum 1000 cases in CI; never reduce).

Regression tier (consistency): `validation/regression/` holds expected output hashes from approved versions. **Never regenerate a fixture hash to make CI pass** — a mismatch means either the change is wrong or `ALGO_VERSION` needs incrementing with a new fixture set.

QC.R integration test (requires R + `jsonlite` + `digest`):

```bash
just qc             # cargo test -p clinrand-package --test qc_r
```

## Rust conventions and checks

- **Edition 2021** with `#![forbid(unsafe_code)]` and `#![deny(clippy::all)]` on all crates.
- **No `.unwrap()` or `.expect()` in library code** — errors are `thiserror`-typed and propagated. Unwrap is okay in tests and `main`.
- **No integer width casts** (`as u32` etc.) in the allocation path — use `try_into()` and handle errors.
- **Checked or saturating arithmetic** in allocation: overflow must never wrap silently.
- **No floating point** anywhere in allocation (no `f32`, `f64`). Ratios, block sizes, counts are integers.
- **RNG pinning**: `rand_chacha` only, exact version (`=x.y.z`). `Cargo.lock` is committed. Forbidden: `thread_rng`, `StdRng`, `SmallRng`, `OsRng` in allocation.
- **Canonical stratum order** is the order given in the config, last factor varying fastest. Do not sort for determinism — config order *is* the determinism.
- **Stream consumption order** (plan §2.4) is contract-bound. Block size drawn before permutation. One RNG per run, shared across strata.
- Public API items carry doc comments. Anything in the determinism contract states it is contract-bound and must not change without `ALGO_VERSION` bump.
- CI enforces `rustfmt` defaults; no custom formatting rules.

## `ALGO_VERSION` and breaking changes

`clinrand-core::ALGO_VERSION` is a plain integer separate from the crate version.

- **Increment it** when anything in the determinism contract changes (RNG, uniform_below, permutation, stream order, stratum order, type definitions in the allocation path, how many bytes are consumed — anything).
- **Do not increment it** for report formatting, CLI flags, UI work, or documentation.
- Never change it as a side effect of another change — it gets its own commit with a changelog entry and a new regression fixture set under `validation/regression/algo-vN/`.
- **`clinrand reproduce` refuses outright** when a manifest's `algo_version` differs from the binary's. Do not add `--force`, `--skip-version-check`, or bypasses.

## Output package format (versioned interface)

`list.csv` columns, manifest fields (plan §6), and canonical JSON rules are consumed by **myIWRS** downstream. Changes to the output format are breaking changes. Treat the format as stable; coordinate with downstream before any change.

## The user and the regulatory context

ClinRand is used by one unblinded statistician on one machine, a handful of times per study. Years later, an inspector may ask that statistician to regenerate the list from its manifest and prove it matches. Design every output with that conversation in mind.

- The tool produces no approved artifact on its own; the list must be independently QC'd and formally approved by a qualified statistician.
- The `qc.R` script and evidence in `validation/` exist to make review possible; responsibility for the list rests with the statistician, not the software.
- The repository is public; that's fine — the algorithm is not the secret, the seed is.

## Testing requirements (non-negotiable)

- ChaCha20 keystream matches RFC 8439 vectors exactly.
- `uniform_below` matches hand-worked rejection-sampling cases, including zero consumption at `n == 1`.
- `generate(cfg, seed)` called twice returns identical bytes (determinism).
- A one-bit seed change produces a different list.
- Property checks P01–P09 (plan §7) hold across the full proptest sweep (1000+ cases).
- Every regression fixture's `list_sha256` matches.
- The blinded report contains no randomization-number-to-arm pairing. Implement as a real test: for every randomization number, assert no line of the rendered report pairs that number with any arm code.
- Emitted `qc.R` runs clean on synthetic packages and reports FAIL on deliberately corrupted `list.csv`.
- `reproduce` refuses algo_version mismatch with exit code 4.

Never mark any of these `#[ignore]`.

When a test fails, assume the code is wrong first. Do not add tolerances, retries, or randomized-seed workarounds — non-determinism in this codebase is a bug by definition. For property test failures on a shrunk case you believe is invalid, fix `validate_config` to reject it instead of narrowing the proptest strategy.

## Git conventions

- One phase per branch, one PR per phase, referencing which plan (§12) requirements it implements.
- Where the plan is ambiguous, write your reading in the PR description and flag it; do not resolve silently.
- Where you believe the plan is wrong, say so before implementing.
- Record any decision not dictated by the plan in `docs/decisions/` as a short numbered entry (decision, alternatives, reason).
- Keep `CHANGELOG.md` current, with prominent entries for any `ALGO_VERSION` change.

## Before you start

Confirm which phase you are in by checking `docs/plans/clinrand-implementation-plan.md` §12 ("Done when" criteria) and `TODO.md`. Do not start a phase until the previous one's criteria are met.

Current status (as of Sept 2026): Phases 1–9 complete (engine, package, CLI, desktop, `qc.R`, security posture, encryption at rest, tagged release installers, validation completeness, operator handbook). Plan §12 (v1 scope) is fully implemented.
