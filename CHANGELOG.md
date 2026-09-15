# Changelog

All notable changes to ClinRand are documented here.

`ALGO_VERSION` is independent of the crate version. A bump of that
integer gets a prominent entry and a new regression fixture set; it
is never changed as a side effect of another change.

## Unreleased

### Added

- Phase 4 package writer complete: `write_package` E2E invariant that two
  runs with the same `(config, seed, meta)` produce byte-identical
  `list.csv` / `stream.csv`, with `checksums.txt` verifying HTML-inclusive
  digests. Phase 4 ticked in `TODO.md`. `ALGO_VERSION` unchanged (1).
- Blinded `generation-report.html` and restricted `unblinded-report.html`
  in `clinrand-package` (`render_generation_report` /
  `render_unblinded_report`), integrated into `write_package` so
  `checksums.txt` covers both HTML files. Property results come from
  `check_properties`. Blinded report emits check id + pass/fail
  (+ informational) only; full `PropertyCheck.detail` text is unblinded-
  only. Per-stratum counts/blocks follow canonical `stratum_combinations`
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
