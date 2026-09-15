# Changelog

All notable changes to ClinRand are documented here.

`ALGO_VERSION` is independent of the crate version. A bump of that
integer gets a prominent entry and a new regression fixture set; it
is never changed as a side effect of another change.

## Unreleased

### Added

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
