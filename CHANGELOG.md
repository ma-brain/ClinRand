# Changelog

All notable changes to ClinRand are documented here.

`ALGO_VERSION` is independent of the crate version. A bump of that
integer gets a prominent entry and a new regression fixture set; it
is never changed as a side effect of another change.

## Unreleased

### Added

- Phase 9 validation completeness and docs complete (plan §12): the
  regression tier is real. New `validation/regression/algo-v1/` — 4
  fixtures (one per `examples/*.json` config) pinning `(config, seed) ->
  config_sha256/list_sha256/stream_sha256/record_count` for the current
  engine. New `clinrand-package` module `regression`
  (`load_regression_cases` / `check_regression_case` — loads and compares
  only, never calls `generate`, matching `write_package`'s convention of
  taking an already-built `GeneratedList`), reused by `clinrand
  validation-report`, the desktop Validation screen, and a new
  automated `cargo test -p clinrand-package --test regression_fixtures`
  (AGENTS.md §8's non-negotiable "every regression fixture's `list_sha256`
  matches" requirement). `clinrand validation-report` now renders real
  PASS/FAIL for all three tiers — the regression tier is never a permanent
  `SKIP`. New `validation/regression/README.md` states the
  never-regenerate-the-hash rule and the fixture format; `validation/README.md`
  updated accordingly. New `handbook/` (`README.md` +
  `operator-workflow.md`): one continuous walkthrough from writing a config
  to a formally approved, archived package, using
  `examples/stratified-block-variable.json` throughout — every command in
  it was run for real against this branch. `ALGO_VERSION` unchanged (1).
  Also fixed two pre-existing documentation/tooling bugs found while
  verifying that walkthrough: the `just cli -- <command>` pattern used
  throughout `README.md`/`docs/cli.md` was broken (the `cli` recipe already
  inserts its own `--`, so typing another one duplicates it and fails) —
  corrected to `just cli <command>` everywhere; and the `cli` recipe's
  `*ARGS` passthrough silently re-split quoted multi-word arguments (e.g.
  `--operator "Jane Statistician"` became two arguments) — fixed with
  `set positional-arguments := true` and `"$@"` in the justfile.
- Phase 8 hardening and release complete: encryption at rest and a tagged
  release workflow (plan §12). `--encrypt` on `generate`/`reproduce` (CLI)
  and an "Encrypt restricted files" option on the desktop Generate screen
  wrap `list.csv`, `list.json`, `manifest.unblinded.json`, `stream.csv`, and
  `unblinded-report.html` into a single `restricted.age` container —
  XChaCha20-Poly1305 AEAD with an Argon2id-derived key
  (`docs/decisions/0007-restricted-container-format.md`,
  `docs/output-package.md` §10). No plaintext restricted file is written
  when `--encrypt` is used. New `clinrand decrypt --package <dir>` CLI
  command and desktop decrypt flow reverse this **in place**, so `qc.R` and
  `verify` work unchanged afterward
  (`docs/decisions/0008-cli-decrypt-command-and-passphrase-ux.md`); CLI
  passphrase entry is a hidden terminal prompt (double-entry on encrypt,
  single on decrypt) with a piped-stdin fallback for scripting and tests.
  New CLI exit code 5 (`PassphraseFailure`). `verify_package` /
  `VerifyReport` gained `properties_checked`, skipping property checks
  (never failing them) for an un-decrypted encrypted package.
  `.github/workflows/release.yml`: a `v*` tag builds unsigned macOS (both
  architectures), Windows NSIS, and Linux `.deb`/AppImage installers via
  `tauri-apps/tauri-action`, publishes them to a GitHub Release, and adds a
  `SHA256SUMS.txt` a downloader can verify with `sha256sum -c`
  (`docs/release.md`, `docs/decisions/0009-unsigned-release-installers.md`).
  `ALGO_VERSION` unchanged (1) — nothing here touches allocation
  determinism; `clinrand-core` is untouched.
- Phase 7 desktop application complete: Tauri 2 + SvelteKit (static SPA,
  `ssr=false`) app with all plan §11.1 screens — config builder with live
  validation, blinded structure preview, generate (with unblinded-material
  confirmation, no encryption control), package viewer with verify and inline
  blinded report, unblinded view behind an explicit confirmation that appends
  a timestamped `access-log.txt` line, validation screen mirroring the three
  tiers with their differing evidential status labelled, and About. Allocation,
  hashing, canonicalization, and validation stay in `clinrand-core` /
  `clinrand-package`; TypeScript only invokes Tauri commands. Capability
  lock-down per §11.2 (only `dialog` + scopeless `fs`; no
  `http`/`shell`/`process`/`updater`; CSP `default-src 'self'`, `connect-src
  'self'`; desktop-only bundle targets). New `docs/security-posture.md`
  documents the no-network posture and how to verify it from the capability
  files. Added a `desktop` CI job (ubuntu) that builds the frontend, `cargo
  check`s the Tauri host, runs the command-flow integration test, and greps the
  capabilities for absence of `http`/`shell`/`process`/`updater`; `justfile`
  `desktop-check`/`desktop-test`/`desktop-dev`/`desktop-caps-check` recipes;
  and a GUI-free Rust integration test exercising validate → preview →
  generate → verify and asserting no seed leaks in the generate outcome.
  Seed never shown in the UI. `ALGO_VERSION` unchanged (1).
- Phase 6 QC complete: study-specific `qc.R` emitted in every package
  (`render_qc_r`, independent stream reconstruction, hashes, P01–P09);
  CI installs R with `jsonlite` and `digest` and runs PASS/FAIL integration
  tests; `docs/qc-procedure.md` documents operator QC, seed handling, and
  optional §9.3 distributional comparison with `blockrand`/`randomizeR`.
  `checksums.txt` includes `qc.R`. `ALGO_VERSION` unchanged (1).
- Phase 5 CLI complete: all plan §10 commands (`list-methods`,
  `validate-config`, `generate`, `reproduce`, `verify`,
  `validation-report`, `version`), global `--json`, exit codes 0–4, and
  `docs/cli.md`. E2E round-trip CI test (library package write → CLI
  `reproduce` → CLI `verify`; `reproduce` refuses `algo_version`
  mismatch with exit 4). `--encrypt` deferred to Phase 8.
  `ALGO_VERSION` unchanged (1).
- Phase 4 final-review hardenings: `write_package` refuses overwrite of an
  existing package directory; E2E calls `generate` twice then writes under
  distinct parent dirs and asserts equal lists plus byte-identical
  `list.csv` / `stream.csv`; manifests/reports take
  `clinrand_core::ENGINE_VERSION` and `RNG_CRATE_VERSION`; blinded report
  states P10 must not cause regeneration and includes arm-code-safe P10
  max-run detail. `PackageMeta::new` / `build_manifests` validate
  `generated_at` as `YYYY-MM-DDTHH:MM:SSZ`. `ALGO_VERSION` unchanged (1).
- Phase 4 package writer complete: `write_package` E2E invariant that two
  runs with the same `(config, seed, meta)` produce byte-identical
  `list.csv` / `stream.csv`, with `checksums.txt` verifying HTML-inclusive
  digests. Phase 4 ticked in `TODO.md`. `ALGO_VERSION` unchanged (1).
- Blinded `generation-report.html` and restricted `unblinded-report.html`
  in `clinrand-package` (`render_generation_report` /
  `render_unblinded_report`), integrated into `write_package` so
  `checksums.txt` covers both HTML files. Property results come from
  `check_properties`. Blinded report emits check id + pass/fail
  (+ informational), a P10 no-regeneration policy sentence, and safe P10
  max-run detail; other `PropertyCheck.detail` text is unblinded-only.
  Per-stratum counts/blocks follow canonical `stratum_combinations`
  order (including zeros). Mandatory blind-safety CI test on a real
  `generate()` DEMO list with delimiter-aware pairing and an unblinded
  negative control; seed hex absent. No templating crate. `ALGO_VERSION`
  unchanged (1).
- `write_package` in `clinrand-package`: writes
  `<study_id>_<compact generated_at>_<list_sha256[:8]>/` with
  `list.csv` / `list.json` / `stream.csv`, both manifests (exact
  `ManifestPair` bytes), both HTML reports, and `checksums.txt` (GNU
  `sha256sum` text mode, sorted by filename, excluding itself). No
  `qc.R`. `ALGO_VERSION` unchanged (1).
- Blinded and unblinded package manifests in `clinrand-package`
  (`build_manifests`): compact canonical JSON plus trailing `\n`;
  `list_sha256` / `stream_sha256` digest exact list/stream CSV UTF-8
  bytes; `seed_sha256` digests raw 32 seed bytes; blinded omits
  `seed_hex` entirely. `ALGO_VERSION` unchanged (1).
- `render_list_csv`, `render_list_json`, and `render_stream_csv` in
  `clinrand-package`: in-memory UTF-8 LF emitters for plan §6 list and
  stream files (no filesystem write). Stratum CSV/JSON columns follow
  config factor order; trailing newline policy documented in
  `docs/output-package.md`. No seed in these outputs. `ALGO_VERSION`
  unchanged (1).
- Property suite (`proptest`, 1000 cases) and named determinism tests in
  `clinrand-core`: arbitrary valid configs must satisfy P01–P09 via
  `all_required_passed`, and `generate` is byte-stable under a fixed seed
  while a one-bit seed flip changes the list. Epistemic note under
  `validation/properties/README.md`.
- `check_properties` / `PropertyReport` in `clinrand-core` for plan §7
  checks P01–P10. P10 is informational only and never fails. P03/P08 exempt
  only truncated final blocks per stratum; non-final under-full blocks fail.
  Simple size-1 blocks skip ratio-bearing P03/P09 rules. P09 uses integer
  one-block tolerance for block methods.
- `generate` in `clinrand-core` for `simple`, `permuted_block`, and
  `stratified_block`: block truncation (§5.5), global and
  `per_stratum_range` numbering (§5.6), one shared RNG/stream per run.
  Numbering width overflow policy in `docs/decisions/0005-*.md`.
  Simple stream order documented in `docs/determinism.md`.
- Descending Fisher–Yates `permute` in `clinrand-core` (plan §2.3) with
  hand-worked reference cases under `validation/reference/fisher-yates/`.
- Canonical stratum Cartesian product `stratum_combinations` (plan §5.4).
- `canonical_json_value` in `clinrand-package`: the same plan §6.4
  canonicalizer over a `serde_json::Value`, so Phase 4 can reuse one
  implementation. `canonical_json(&StudyConfig)` calls it.
- Canonical JSON and `config_sha256` in `clinrand-package` (plan §6.4):
  object keys sorted by UTF-8 byte order, compact encoding, SHA-256 of
  those UTF-8 bytes as lowercase hex. Documented in
  `docs/output-package.md`. Plan §4 sketched `canonical_json` on
  `clinrand-core`; hashing stays in the package crate.
- Published study-config JSON Schema at `docs/schema/study-config-1.0.json`
  and synthetic configs under `examples/` (wire format, plan §5.1).
- `validate_config` in `clinrand-core` with every plan §5.2 reject
  rule, plus warnings for non-multiple `list_length_per_stratum` and
  `per_stratum_range` numbering disclosure (plan §5.6).
- `StudyConfig` types and JSON serde in `clinrand-core`, matching plan
  §4 / §5.1 (`method` string plus sibling `block` / `numbering`).
- Phase 0 workspace scaffold: `clinrand-core`, `clinrand-package`,
  `clinrand-cli`, `justfile`, and CI on Linux, macOS, and Windows.
- ChaCha20 generator in `clinrand-core` (`rand_chacha = "=0.3.1"`)
  and RFC 8439 §2.3.2 / §2.4.2 reference cases under
  `validation/reference/chacha20/`.
- `uniform_below` rejection sampling, `StreamLog` recording, and
  hand-worked cases under `validation/reference/uniform-below/`.
- `docs/determinism.md` — normative RNG-layer contract (plan §2.1–2.6).

### Changed

- Plan §5.2 and `validate_config` reject `per_stratum_range` when
  `block_size < list_length_per_stratum` (`ConfigError::PerStratumRangeTooSmall`).
  Disclosure remains a warning. Decision 0006 updated.
- `validate_config` rejects block size `0` (`ConfigError::BlockSizeZero`)
  for fixed `size` and any entry in variable `sizes`. Size `0` is a
  multiple of every ratio sum (`0 % n == 0`) and was previously accepted.
- Study-config serde uses `#[serde(deny_unknown_fields)]` on the wire
  types, matching schema `additionalProperties: false`.
- Published schema `study-config-1.0.json` now encodes structural §5.2
  bounds JSON Schema can express: `arms.minItems` 2, `ratio.minimum` 1,
  block `size`/`sizes[]` `minimum` 1, variable `sizes` `maximum` 24,
  `uniqueItems`, and `maxItems` 24. Ratio-sum multiples stay in
  `validate_config`.
