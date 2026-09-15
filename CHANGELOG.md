# Changelog

All notable changes to ClinRand are documented here.

`ALGO_VERSION` is independent of the crate version. A bump of that
integer gets a prominent entry and a new regression fixture set; it
is never changed as a side effect of another change.

## Unreleased

### Added

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
