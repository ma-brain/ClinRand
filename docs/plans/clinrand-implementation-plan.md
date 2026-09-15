# ClinRand — Implementation Plan

**Status:** approved scope, ready for implementation
**Target repo:** `github.com/ma-brain/ClinRand` (public)
**Sibling project:** `ma-brain/ClinSize` — follow its structural conventions unless this document says otherwise
**Downstream consumer:** `myIWRS` (separate web application, not in scope here)

---

## 0. How to use this document

This plan is written to be executed by an implementing agent with limited context. Rules of engagement:

1. Work phase by phase in the order given in §12. Do not start a phase until the previous phase's "Done when" criteria are all met.
2. Where this document specifies a name, a format, a file path, or an algorithm, treat it as binding. Do not substitute equivalents.
3. Where this document does not specify something, prefer the simplest option that satisfies §13 (Guardrails).
4. Never silently change anything listed in §2 (Determinism contract). Changes there require an explicit version bump documented in the changelog.
5. If a requirement here appears to conflict with another, stop and flag it rather than choosing.

---

## 1. Purpose and scope

ClinRand is an **offline desktop application and command-line tool** that generates randomization lists for clinical trials, together with the evidence package needed to QC, approve, and hand that list over to an IWRS.

It is used by an **unblinded statistician**, on a single machine, a handful of times per study. It is not a server, not multi-user, and never touches a network.

### In scope for v1

- Simple, permuted-block, and stratified permuted-block randomization
- Deterministic, reproducible generation from a recorded seed
- A self-contained output package: list, manifest, random stream, QC script, blinded and unblinded reports
- Built-in property checks on every generated list
- A three-tier validation suite runnable from CLI and CI
- Desktop UI (Tauri 2 + SvelteKit) and CLI (`clinrand`), both calling one shared Rust engine

### Explicitly out of scope for v1

| Excluded | Reason |
|---|---|
| Runtime / on-demand allocation | That is myIWRS's job |
| Minimization, biased-coin, big-stick, maximal procedure, response-adaptive designs | Runtime algorithms; heavier validation burden; deferred to v2 |
| Drug supply, kit lists, medication numbering | Separate domain model; deferred |
| Any network capability, telemetry, auto-update | Regulatory posture: the tool must be provably incapable of transmitting a list |
| Mobile targets (Tauri iOS/Android) | A randomization list generator on a phone is an audit finding |
| Multi-user accounts, server deployment | Single-operator desktop tool by design |
| List extension / top-up after go-live | Needs myIWRS design decisions first |

---

## 2. Determinism contract

**This section is the heart of the product. Everything here is binding.**

A given `(config, seed)` pair must produce a byte-identical list on every platform, on every run, forever — until `algo_version` is deliberately incremented.

### 2.1 Random number generator

- Use `rand_chacha::ChaCha20Rng`, seeded from a 256-bit seed.
- Pin the crate with an exact version requirement (`rand_chacha = "=0.3.1"` or the current stable equivalent) and commit `Cargo.lock`.
- **Do not** use `rand::thread_rng`, `StdRng`, `SmallRng`, or any OS/thread RNG in the allocation path. `StdRng`'s backing algorithm has changed across `rand` major versions; that class of drift is exactly what this rule prevents.
- `getrandom` is used in **one place only**: drawing a fresh seed at the start of a generation run.

### 2.2 Uniform integer draws

Implement the engine's own `uniform_below(rng, n) -> u64` using **rejection sampling**, so behaviour does not depend on the `rand` crate's distribution internals:

1. If `n == 0`, return an error. If `n == 1`, consume **zero** bytes and return 0.
2. Compute `zone = u64::MAX - (u64::MAX % n)`.
3. Draw `x = rng.next_u64()`. If `x >= zone`, discard and redraw. Otherwise return `x % n`.

Document this in `docs/determinism.md` in enough detail that a third party can reimplement it.

### 2.3 Permutation

Fisher–Yates, descending index:

```
for i in (1..len).rev():
    j = uniform_below(rng, i + 1)
    swap(items[i], items[j])
```

### 2.4 Stream consumption order

The order in which the engine consumes randomness is part of the contract. For each stratum, in the canonical stratum order defined in §5.4, and for each block in ascending block index:

1. If block sizes are variable: draw the block size first — `uniform_below(rng, sizes.len())` indexing into the sorted, deduplicated `sizes` array.
2. Build the block's arm multiset from the allocation ratio.
3. Permute it with Fisher–Yates per §2.3.

One `ChaCha20Rng` instance per **run**, not per stratum. Strata are processed sequentially in canonical order and share the stream.

### 2.5 No floating point

The allocation path must contain no `f32` or `f64`. Allocation ratios, block sizes, and counts are integers. Floats may appear only in presentational summaries in reports, never in anything that influences allocation or hashing.

### 2.6 Version identity

Every generated package records:

- `engine_version` — the crate version of `clinrand-core`, e.g. `0.3.1`
- `algo_version` — a plain integer, starting at `1`

`algo_version` **must** be incremented whenever anything in §2.1–2.5 changes, including changes that only affect the number or order of bytes consumed. A change to report formatting, CLI flags, or the UI does not increment it. This is the single most important invariant in the project: `algo_version` is what tells a future operator whether an old manifest can still be reproduced.

**Decision — why this is a separate integer, not the crate version.** The crate version moves for every release, including cosmetic ones. The question a manifest has to answer years later is narrower: *can this binary still produce the exact list recorded here?* That depends only on §2.1–2.5. Using the crate version as the marker would make every harmless release appear to invalidate old manifests. Using no marker would be worse: a silent change to the stream logic would produce a different list on regeneration with no warning, and an operator could believe a list had been reproduced when it had not. A dedicated integer that moves only with the allocation logic is the only option that answers the question correctly in both directions.

This is also why `clinrand reproduce` refuses outright on a mismatch rather than proceeding (§10, exit code 4). If the manifest records `algo_version: 1` and the binary is at 2, that binary cannot produce that list. The only acceptable outcomes are a byte-identical regeneration or an explicit refusal naming the required version. Emitting a plausible but different list is never acceptable, and the implementing agent must not add a `--force` flag that permits it.

---

## 3. Repository layout

```
ClinRand/
├── apps/
│   └── desktop/              SvelteKit + Tauri 2 application
├── crates/
│   ├── clinrand-core/        Engine. No UI, no Tauri, no I/O, no clock
│   ├── clinrand-package/     Package reading/writing, hashing, reports
│   └── clinrand-cli/         The `clinrand` binary
├── validation/
│   ├── reference/            External normative evidence
│   ├── properties/           Property-based invariants
│   ├── regression/           Frozen golden fixtures
│   └── README.md             Epistemic status of each tier — REQUIRED
├── examples/                 Synthetic study configs
├── docs/
│   ├── determinism.md
│   ├── output-package.md
│   ├── qc-procedure.md
│   └── plans/
├── handbook/                 User-facing documentation
├── .github/workflows/
├── Cargo.toml                Workspace root
├── Cargo.lock                COMMITTED
├── justfile
├── LICENSE
├── README.md
└── TODO.md
```

### 3.1 Crate boundaries

| Crate | May depend on | Must not |
|---|---|---|
| `clinrand-core` | `rand_chacha`, `serde`, `thiserror` | Touch the filesystem, read the clock, read env vars, call `getrandom`, know what a file path is |
| `clinrand-package` | `clinrand-core`, `serde_json`, `sha2`, `chrono`, templating | Implement any allocation logic |
| `clinrand-cli` | both above, `clap` | Implement any allocation or hashing logic |
| `apps/desktop` | `clinrand-core`, `clinrand-package` via Tauri commands | Implement any allocation logic in TypeScript |

Add `#![forbid(unsafe_code)]` to every crate.

**Rule inherited from ClinSize:** allocation logic lives in `clinrand-core`. The UI and CLI never reimplement it.

---

## 4. Core public API

Sketch — adjust field names only if a genuine conflict arises, and keep them identical to the JSON schema in §5.

```rust
// ---- Configuration ----

pub struct StudyConfig {
    pub schema_version: String,
    pub study_id: String,
    pub protocol_version: String,
    pub arms: Vec<Arm>,
    pub method: Method,
    pub strata: Vec<StratificationFactor>,
    pub list_length_per_stratum: u32,
    pub numbering: NumberingScheme,
}

pub struct Arm { pub code: String, pub label: String, pub ratio: u32 }

pub enum Method {
    Simple,
    PermutedBlock { block: BlockScheme },
    StratifiedBlock { block: BlockScheme },
}

pub enum BlockScheme {
    Fixed { size: u32 },
    Variable { sizes: Vec<u32> },
}

pub struct StratificationFactor { pub name: String, pub levels: Vec<String> }

pub enum NumberingScheme {
    Global { start: u32, width: u8 },
    PerStratumRange { start: u32, block_size: u32, width: u8 },
}

// ---- Output ----

pub struct GeneratedList { pub records: Vec<AllocationRecord>, pub stream: StreamLog }

pub struct AllocationRecord {
    pub randomization_number: String,
    pub stratum: BTreeMap<String, String>,   // factor name -> level
    pub block_id: u32,
    pub block_size: u32,
    pub position_in_block: u32,
    pub arm_code: String,
}

pub struct StreamLog { pub draws: Vec<StreamDraw> }

pub struct StreamDraw { pub index: u64, pub bound: u64, pub value: u64, pub purpose: DrawPurpose }

pub enum DrawPurpose { BlockSize, Permutation, SimpleAllocation }

// ---- Entry points ----

pub fn validate_config(cfg: &StudyConfig) -> Result<(), Vec<ConfigError>>;
pub fn generate(cfg: &StudyConfig, seed: [u8; 32]) -> Result<GeneratedList, GenerationError>;
pub fn check_properties(list: &GeneratedList, cfg: &StudyConfig) -> PropertyReport;
pub fn canonical_json(cfg: &StudyConfig) -> String;
pub const ALGO_VERSION: u32 = 1;
```

`generate` is a pure function. It takes no clock, no paths, no operator name.

---

## 5. Study configuration

### 5.1 JSON schema (v1.0)

```json
{
  "schema_version": "1.0",
  "study_id": "ABC-201",
  "protocol_version": "2.1",
  "arms": [
    { "code": "A", "label": "Investigational product 50 mg", "ratio": 2 },
    { "code": "P", "label": "Placebo", "ratio": 1 }
  ],
  "method": "stratified_block",
  "block": { "kind": "variable", "sizes": [6, 9] },
  "strata": [
    { "name": "site", "levels": ["001", "002", "003"] },
    { "name": "agegrp", "levels": ["LT65", "GE65"] }
  ],
  "list_length_per_stratum": 36,
  "numbering": { "kind": "global", "start": 10001, "width": 5 }
}
```

Publish this as a JSON Schema file at `docs/schema/study-config-1.0.json` and validate against it in `validate_config`.

### 5.2 Validation rules

`validate_config` must reject, with a specific error per failure:

- `arms.len() < 2` (unless `method == simple` with a single arm, which is also rejected)
- Duplicate arm codes; arm code not matching `^[A-Za-z0-9_-]{1,12}$`
- Any `ratio == 0`
- Fixed block size not a multiple of `sum(ratios)`
- Any variable block size not a multiple of `sum(ratios)`
- `sizes` empty, containing duplicates, or containing a value `> 24`
- `list_length_per_stratum` not a multiple of `sum(ratios)` — warn, not reject, but the final block may be truncated; see §5.5
- Duplicate stratum factor names; duplicate levels within a factor
- Factor or level names not matching `^[A-Za-z0-9_.-]{1,32}$`
- Total stratum combinations `> 200` (sanity guard; configurable override flag)
- `method == stratified_block` with an empty `strata` array
- `method == permuted_block` with a non-empty `strata` array

### 5.3 Stratum combinations

The stratum space is the full Cartesian product of all factors' levels. Every combination receives its own independent block sequence of `list_length_per_stratum` allocations.

### 5.4 Canonical stratum order

Factors are ordered **as given in the config array** (not sorted). Levels are ordered **as given in each factor's array**. The Cartesian product is enumerated with the **last factor varying fastest**.

This ordering is part of the determinism contract (§2.4). Do not sort it.

### 5.5 Block truncation

If `list_length_per_stratum` is not an exact multiple of the block sizes drawn, the final block in each stratum is truncated to fit. The truncated block is still generated in full (consuming the full stream) and then cut — this keeps the stream consumption predictable. Flag truncated blocks in the report; the property check for "exact ratio at block boundary" skips the final block of each stratum when truncated.

### 5.6 Numbering

- `global` — one ascending counter across the whole list, assigned in canonical stratum order (§5.4), then block order, then position order. Format `{start + n}` zero-padded to `width`.
- `per_stratum_range` — each stratum combination gets a reserved contiguous range starting at `start + (stratum_index * block_size)`.

**Decision — `global` is the default; `per_stratum_range` is opt-in.**

With reserved per-stratum ranges, the randomization number itself discloses stratum membership. If site 002 / age ≥65 owns 20001–20036, then anyone seeing subject 20014 knows both the site and the age group. Where a stratification factor is a baseline characteristic, that discloses it to blinded assessors who should be evaluating outcomes without it, and it makes block boundaries easier to infer from a sequence of enrolled numbers. Global numbering discloses nothing beyond enrolment order.

Reserved ranges remain supported because they are operationally useful — site-shipped pre-printed kits, and sponsors or downstream systems that mandate site-based number ranges. But the choice must be deliberate:

- `validate_config` accepts `per_stratum_range` without error.
- `clinrand generate` prints a warning to stderr naming the disclosure risk whenever `per_stratum_range` is used, and records the warning in both reports.
- The desktop config builder shows the same warning inline next to the field, not in a dismissible toast.
- No configuration may set `per_stratum_range` implicitly or by default.

---

## 6. The output package

This is the **ingest contract with myIWRS**. Treat it as a published interface.

Written to `<out_dir>/<study_id>_<UTC timestamp>_<first 8 hex of list_sha256>/`:

```
ABC-201_20260915T144210Z_a3f91c02/
├── manifest.blinded.json        circulates freely
├── manifest.unblinded.json      restricted
├── list.csv                     restricted — the allocations
├── list.json                    restricted — same content, structured
├── stream.csv                   restricted — consumed random draws
├── qc.R                         emitted QC script (§9)
├── generation-report.html       BLINDED — no allocations anywhere
├── unblinded-report.html        restricted — full list rendered
└── checksums.txt                SHA-256 of every file above
```

### 6.1 `list.csv`

Header row, then one row per allocation, in canonical order:

```
randomization_number,<one column per stratum factor>,block_id,block_size,position_in_block,arm_code
```

UTF-8, LF line endings, no BOM, no quoting unless a field contains a comma. Deterministic — the same generation always produces a byte-identical file.

### 6.2 `manifest.unblinded.json`

```json
{
  "schema_version": "1.0",
  "study_id": "ABC-201",
  "protocol_version": "2.1",
  "generated_at": "2026-09-15T14:42:10Z",
  "operator": "Name Surname",
  "config": { "...": "the full StudyConfig, verbatim" },
  "config_sha256": "…",
  "seed_hex": "…64 hex chars…",
  "seed_sha256": "…",
  "rng": { "algorithm": "ChaCha20", "crate": "rand_chacha", "crate_version": "0.3.1" },
  "engine_version": "0.3.1",
  "algo_version": 1,
  "record_count": 216,
  "list_sha256": "…",
  "stream_sha256": "…"
}
```

### 6.3 `manifest.blinded.json`

Identical **except** `seed_hex` is omitted entirely. Everything else — including `seed_sha256`, all hashes, and the full config — is retained.

The seed is functionally equivalent to the list itself: anyone holding the seed and the config can regenerate every allocation. Treat `manifest.unblinded.json` with exactly the same care as `list.csv`.

### 6.4 Canonical JSON

Define once, in `clinrand-package`, and use for every hash:

- Object keys sorted lexicographically by UTF-8 byte order
- No insignificant whitespace
- Integers rendered without decimal point or exponent
- Strings with minimal escaping, UTF-8
- No trailing newline

`config_sha256` is the SHA-256 of the canonical JSON of the config object. Write `docs/output-package.md` describing this precisely enough for an independent reimplementation.

### 6.5 `generation-report.html` must be blind-safe

This is a hard requirement with a test to match. The blinded report contains:

- Study ID, protocol version, generation timestamp, operator
- Full configuration: arms and **ratios**, method, block scheme, strata and levels, numbering scheme
- Number of records, per stratum
- Block structure: how many blocks, of which sizes, per stratum
- `seed_sha256`, all file hashes, engine and algo versions
- Property check results (§7), pass/fail per check
- Truncation warnings

It contains **no arm assignment for any randomization number**, no seed, and no per-block composition beyond the ratio implied by the config. Write a CI test that greps the rendered blinded report for every randomization number in the list and asserts no line containing one also contains an arm code.

### 6.6 Encryption

`--encrypt` wraps the restricted files (`list.*`, `stream.csv`, `manifest.unblinded.json`, `unblinded-report.html`) into `restricted.age`-style container using XChaCha20-Poly1305 with an Argon2id-derived key from an operator passphrase. Plaintext originals are not written when `--encrypt` is used. Phase 8 — do not block earlier phases on it.

---

## 7. Property checks

Run automatically on every generation; results embedded in both reports. Each returns pass/fail plus detail.

| ID | Check |
|---|---|
| P01 | Record count equals `list_length_per_stratum × number of stratum combinations` (allowing for truncation) |
| P02 | Every block size appears in the configured allowed set |
| P03 | Every complete block contains exactly `ratio[i] × (block_size / sum(ratios))` of each arm |
| P04 | No duplicate randomization numbers |
| P05 | Randomization numbers are contiguous and ascending within their scheme |
| P06 | Every stratum combination in the Cartesian product is present |
| P07 | No stratum combination contains records from another stratum |
| P08 | `position_in_block` is 1..block_size, complete and without gaps, for every block |
| P09 | Overall arm counts match the allocation ratio within one block's tolerance |
| P10 | Maximum run length of identical consecutive arms, per stratum — **informational only**, reported never failed |

P10 is reported because a long run can prompt a methodological conversation, but it is a legitimate outcome of correct randomization and must not be treated as an error or regenerated away. State this explicitly in the report text.

---

## 8. Validation suite

Mirror ClinSize's `validation/` mechanism, but with three tiers of clearly different evidential weight. `validation/README.md` must state this distinction plainly — it is the single most important document in the repo for an auditor.

### 8.1 `validation/reference/` — external normative

The only tier with ClinSize-grade evidence: expected values come from outside the project and were never produced by this engine.

- `chacha20/` — the ChaCha20 test vectors from RFC 8439 §2.3.2 and §2.4.2. Assert the seeded generator reproduces the keystream exactly.
- `uniform-below/` — hand-worked rejection-sampling cases with a fixed keystream, computed by hand and documented in `reference-output.md`.
- `fisher-yates/` — worked permutation examples over a fixed draw sequence, computed by hand.

Each folder carries `cases.json` and `reference-output.md`, following ClinSize's convention, where `reference-output.md` cites the source.

### 8.2 `validation/properties/` — invariants

`proptest`-based. Generate arbitrary valid configs (2–4 arms, ratios 1–3, fixed and variable blocks, 0–3 strata factors with 2–5 levels each) and assert P01–P09 hold for every one. Minimum 1000 cases in CI.

Also assert: `generate(cfg, seed)` called twice returns identical output; and a one-bit change in the seed produces a different list.

### 8.3 `validation/regression/` — frozen fixtures

Synthetic configs with fixed seeds and the expected `list_sha256`. Any change to stream consumption breaks these tests.

**Critical:** a broken regression fixture is never fixed by regenerating the expected hash. It is fixed by either reverting the change or incrementing `ALGO_VERSION` and adding a new fixture set under `regression/algo-v2/`, keeping the v1 fixtures in place. Write this rule at the top of `validation/regression/README.md`.

### 8.4 No real study data — ever

The repo is public. Every config, list, and fixture in it is synthetic. Add a CI check that fails if any file under `examples/` or `validation/` contains a `study_id` not matching `^(DEMO|TEST|EXAMPLE)-`.

---

## 9. Emitted R QC script (`qc.R`)

The generator emits a study-specific R script as part of every package. This extends ClinSize's "show the R alongside the result" into "emit the R that re-derives the result."

### 9.1 Required behaviour

The script must, in order:

1. Read `manifest.unblinded.json`, `list.csv`, and `stream.csv` from its own directory.
2. Re-derive the canonical stratum order from the config, independently of anything in `list.csv`.
3. Rebuild every block from `stream.csv` by independently reimplementing block-size selection (§2.4) and Fisher–Yates (§2.3) in R, consuming the recorded draws in order.
4. Compare the reconstructed allocation, row for row, against `list.csv`. Report the first discrepancy if any.
5. Recompute the SHA-256 of `list.csv` and `stream.csv` and compare against the manifest.
6. Recompute `config_sha256` from the manifest's config object using the same canonicalization.
7. Independently re-run property checks P01–P09.
8. Print a PASS/FAIL line per check, an overall verdict, and `sessionInfo()`.

### 9.2 Constraints

- Dependencies limited to base R plus `jsonlite` and `digest`. No tidyverse — the script must run on a locked-down validated R installation.
- The script **must not** reimplement ChaCha20. It consumes the recorded stream. What it independently verifies is the permutation and blocking logic; the RNG is verified separately against RFC 8439 in `validation/reference/`.
- The script is generated from a template with the study's parameters interpolated, so the reviewer reads concrete values rather than abstractions.
- It must exit non-zero on any failure so it can run unattended.

### 9.3 Supplementary independent check (documented, not emitted)

`docs/qc-procedure.md` describes an optional second path: generate a list for the same design with `blockrand` or `randomizeR` and compare **distributional properties only** — arm balance, block size distribution, permutation frequency over many replicates. This cannot produce an identical sequence and must not be presented as if it could.

---

## 10. CLI surface

```
clinrand list-methods
clinrand validate-config --config <file>
clinrand generate --config <file> --out <dir> --operator <name> [--encrypt] [--allow-large-strata]
clinrand reproduce --manifest <manifest.unblinded.json> --out <dir>
clinrand verify --package <dir>
clinrand validation-report [--tier reference|properties|regression] [--format md|html]
clinrand version
```

- `reproduce` regenerates from a manifest and asserts the new `list_sha256` matches the recorded one. It **must refuse** and exit 4 if the manifest's `algo_version` differs from the binary's `ALGO_VERSION`, with a message naming both values.
- `verify` recomputes every hash in `checksums.txt` and re-runs property checks against `list.csv` without regenerating.
- Exit codes: `0` success · `1` check failure · `2` invalid config · `3` I/O error · `4` algo version mismatch.
- All commands accept `--json` for machine-readable output.

---

## 11. Desktop application

SvelteKit + Tauri 2, calling `clinrand-core` and `clinrand-package` through Tauri commands. No allocation or hashing logic in TypeScript.

### 11.1 Screens

1. **Config builder** — arms and ratios, method, block scheme, stratification factors and levels, list length, numbering. Live validation using `validate_config`, with errors shown against the offending field.
2. **Structure preview** — blinded only. Shows stratum combinations, block counts and sizes, total records, per-arm totals implied by the ratio. Never shows a generated allocation.
3. **Generate** — operator name, destination folder (chosen via the OS dialog), optional encryption. Confirmation dialog stating that the output will contain unblinded material.
4. **Package viewer** — open an existing package, run `verify`, display the blinded report inline.
5. **Unblinded view** — separate screen, reached only through an explicit confirmation dialog. Opening it appends an entry to a local access log stored beside the package.
6. **Validation** — mirrors `validation-report`, showing all three tiers with their differing evidential status clearly labelled.
7. **About** — engine version, algo version, RNG identity, scope disclaimer.

### 11.2 Tauri capability lock-down

In `tauri.conf.json` and the capability files:

- No `http` capability. No `shell`. No `updater`. No `process`.
- `fs` scoped to paths returned by the `dialog` plugin only — no static path allowlist, no `$HOME` wildcard.
- CSP with `default-src 'self'`, no `connect-src` beyond `'self'`, no remote fonts or scripts. Bundle all assets.
- `"bundle": { "targets": ["dmg", "app", "nsis", "deb", "appimage"] }` — desktop only, no mobile targets configured.

Write `docs/security-posture.md` stating that the application has no network capability compiled in, and describing how a reviewer can verify that from the capability files.

---

## 12. Phased delivery

Each phase ends with its "Done when" criteria fully met and CI green.

### Phase 0 — Scaffold
Cargo workspace with three crates; `justfile` with `setup`, `dev`, `test`, `lint`, `build`, `cli`; GitHub Actions running fmt, clippy `-D warnings`, and tests on Linux/macOS/Windows; `README.md` with scope and the §14 disclaimer; `LICENSE`; `TODO.md`.
**Done when:** `just test` and `just lint` pass on a trivial placeholder; CI green on all three OSes.

### Phase 1 — RNG layer
`ChaCha20Rng` wrapper, `uniform_below` with rejection sampling, `StreamLog` recording. `validation/reference/chacha20/` and `uniform-below/` populated and asserted.
**Done when:** RFC 8439 vectors pass; hand-worked rejection cases pass; `docs/determinism.md` written.

### Phase 2 — Config and canonicalization
`StudyConfig` types, `validate_config` with every rule in §5.2, JSON Schema file, canonical JSON, SHA-256 helpers.
**Done when:** every validation rule has a failing-case test; canonical JSON is stable across key insertion orders; schema file validates all `examples/`.

### Phase 3 — Generation engine
Simple, permuted-block, stratified-block. Canonical stratum ordering. Block truncation. Numbering schemes. Property checks P01–P10.
**Done when:** proptest suite (§8.2) passes 1000+ cases; determinism test passes; `generate` is pure (no I/O in the crate — enforce by having no `std::fs` import in `clinrand-core`).

### Phase 4 — Package writer
`clinrand-package`: list.csv, list.json, stream.csv, both manifests, checksums.txt, blinded and unblinded HTML reports.
**Done when:** blind-safety CI test (§6.5) passes; two runs of the same `(config, seed)` produce byte-identical `list.csv`; `checksums.txt` verifies.

### Phase 5 — CLI
All commands in §10, exit codes, `--json` output, `validation-report`.
**Done when:** `generate` → `reproduce` → `verify` round-trip is green in CI on a synthetic study; `reproduce` correctly refuses an `algo_version` mismatch.

### Phase 6 — R QC script
Template and emission; `docs/qc-procedure.md`.
**Done when:** the emitted script runs clean on a synthetic package in CI (R + `jsonlite` + `digest` only) and correctly reports FAIL on a deliberately corrupted `list.csv`.

### Phase 7 — Desktop application
All screens in §11.1; capability lock-down per §11.2.
**Done when:** a full config → preview → generate → verify flow works on all three OSes; `docs/security-posture.md` written; grep of the built bundle confirms no HTTP capability.

### Phase 8 — Hardening and release
Encryption at rest; tagged release workflow producing macOS (both architectures), Windows NSIS, Linux `.deb` and AppImage; SHA-256 published per installer.
**Done when:** a `v*` tag produces attached installers; a downloaded installer's hash matches the published one.

### Phase 9 — Validation completeness and docs
`validation/README.md` with the three-tier epistemic statement; `validation/regression/README.md` with the never-regenerate-the-hash rule; `handbook/` covering operator workflow, QC procedure, and approval expectations.
**Done when:** `clinrand validation-report` renders all three tiers; handbook covers a complete study from config to approved package.

---

## 13. Guardrails for the implementing agent

Do not do any of the following without an explicit instruction from the project owner:

1. Do not add any network capability, HTTP client, telemetry, crash reporting, or auto-updater.
2. Do not use `thread_rng`, `StdRng`, `SmallRng`, or `rand::random` anywhere.
3. Do not change stream consumption order, the permutation algorithm, or the uniform draw method without incrementing `ALGO_VERSION` and adding a new regression fixture set.
4. Do not regenerate a regression fixture's expected hash to make CI pass.
5. Do not introduce floating-point arithmetic into the allocation path.
6. Do not put allocation, hashing, or canonicalization logic in the CLI or the frontend.
7. Do not commit any real study configuration, list, or seed. Synthetic only, with `DEMO-`/`TEST-`/`EXAMPLE-` study IDs.
8. Do not implement minimization, dynamic allocation, or any runtime algorithm.
9. Do not add Tauri mobile targets.
10. Do not place any allocation in the blinded report.
11. Do not invent expected values for `validation/reference/` — they come from a citable external source or the case does not belong in that tier.
12. Do not add dependencies to `qc.R` beyond base R, `jsonlite`, and `digest`.

---

## 14. README scope statement

The README must carry a scope statement that is sharper than ClinSize's, because the output enters a regulated trial. Required substance:

> ClinRand generates randomization lists and the evidence needed to review them. It is not a certified computerised system and produces no approved artifact on its own. A list generated by ClinRand must be independently QC'd and formally approved by a qualified unblinded statistician before any use in a clinical trial. The QC procedure in `docs/qc-procedure.md` and the evidence under `validation/` exist to make that review possible; responsibility for the list rests with the statistician who approves it, not with the software.

---

## 15. Deferred to the myIWRS design discussion

Not decisions for this project, but they will constrain the package format later. Keep the output schema versioned so it can evolve.

- Randomization number allocation strategy across sites and strata
- Kit / medication numbering lists and their relationship to the randomization list
- List extension or top-up after a study has started allocating
- Multi-list studies (separate lists per cohort, period, or re-randomization point)
- Whether myIWRS ingests the whole package or only `list.csv` plus `manifest.blinded.json`
- How the algo-version gate behaves on the ingest side
