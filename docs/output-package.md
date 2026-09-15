# Output package — canonical JSON, hashing, and list/stream renderers

This document specifies how ClinRand produces **canonical JSON**,
**`config_sha256`**, in-memory **`list.csv` / `list.json` /
`stream.csv`** byte layouts, **manifests**, **HTML reports**, and the
**`write_package`** filesystem layout including **`checksums.txt`**.
Canonical JSON is precise enough for an independent reimplementation
of the config hash. `qc.R` emission is specified in §8.

A third party who follows this file, without reading the Rust sources,
must obtain the same canonical bytes and the same SHA-256 for a given
`StudyConfig`.

---

## 1. Where this lives

Canonicalization and hashing are implemented in **`clinrand-package`**,
not in `clinrand-core` and not in the CLI.

Plan §4 sketched `canonical_json` on `clinrand-core`. That sketch is
overridden: plan §6.4 places the definition in `clinrand-package`, and
the crate-boundary table forbids hashing in core. Core remains free of
SHA-256 and of any “what a path is” knowledge.

Public API:

- `canonical_json(&StudyConfig) -> Result<String, CanonicalError>`
- `canonical_json_value(&serde_json::Value) -> Result<String, CanonicalError>` —
  the same canonicalization applied to an already-parsed JSON value, so
  Phase 4 can reuse one implementation for package files.
  `canonical_json` serializes the typed config and calls this.
- `config_sha256(&StudyConfig) -> Result<String, CanonicalError>` —
  **lowercase hex SHA-256 of the canonical UTF-8 bytes**, 64 characters,
  no `0x` prefix
- `config_sha256_digest(&StudyConfig) -> Result<[u8; 32], CanonicalError>` —
  the same digest as raw bytes
- `render_list_csv` / `render_list_json` / `render_stream_csv` — see §6
- `build_manifests` — see §7
- `write_package` / `compact_generated_at` — see §8
- `render_generation_report` / `render_unblinded_report` — see §9

`ALGO_VERSION` is not involved. Changing canonicalization changes
hashes in the package; it does not change the allocation stream.

---

## 2. Input: the typed config, not the file text

`config_sha256` is the hash of the **canonical JSON of the config
object** after it has been deserialized into `StudyConfig` (plan §4 /
§5.1 wire format).

Consequences:

- Key order, insignificant whitespace, and Unicode escape choices in an
  input file do not affect the hash.
- Unknown properties fail deserialize (`#[serde(deny_unknown_fields)]`
  on the wire types, matching schema `additionalProperties: false`).
  They cannot be silently dropped from the hashed typed config.
- Optional `block` is omitted entirely when `method` is `"simple"`
  (`skip_serializing_if` on `None`). It is not serialized as `null`.
- `method` is a JSON string (`"simple"`, `"permuted_block"`,
  `"stratified_block"`). When a block scheme is required, `block` is a
  **sibling** object, not nested inside `method`.

Do **not** hash by sorting `StudyConfig` field declaration order and
emitting struct-order JSON. Serialize to a JSON value tree, then
canonicalize that tree.

---

## 3. Canonical JSON (plan §6.4)

Start from `serde_json::Value` produced by serializing `StudyConfig`
(pinned `serde_json = "=1.0.151"`). Then apply the following
recursively.

### 3.1 Objects

- Sort keys **lexicographically by UTF-8 byte order** (unsigned byte
  comparison of the key’s UTF-8 encoding). This is **not** Unicode code
  point order when those differ; for JSON keys that are ASCII it
  coincides with ASCII order.
- Canonicalize each member value recursively.
- Emit `{` then `canonical_key` `:` `canonical_value` for each member,
  members separated by a single `,`, then `}`.
- Empty object is `{}`.

Only object keys are sorted. **Do not sort array elements.**

### 3.2 Arrays

- Keep element order exactly as in the JSON value (config order for
  `arms`, `strata`, factor `levels`, variable block `sizes`).
- Canonicalize each element recursively.
- Emit `[` then elements separated by a single `,`, then `]`.
- Empty array is `[]`.

### 3.3 Numbers

Integers only: base-10, no `+` sign, no leading zeros (except the
number `0` itself), **no decimal point, no exponent**.

A non-integer JSON number is an error. `StudyConfig` uses integer
fields (`u32`, `u8`); a correct serialization never hits this.

### 3.4 Strings

UTF-8. Quote with `"`. Minimal escaping:

| Code point | Encoding |
|---|---|
| U+0022 quotation mark | `\"` |
| U+005C reverse solidus | `\\` |
| U+0008 backspace | `\b` |
| U+000C form feed | `\f` |
| U+000A line feed | `\n` |
| U+000D carriage return | `\r` |
| U+0009 tab | `\t` |
| U+0000–U+001F other controls | `\u00XX` with two **lowercase** hex digits |
| All other characters, including `/` (U+002F) and non-ASCII | raw UTF-8, not `\u` escaped |

This matches RFC 8259 with the short two-character escapes above.
Solidus is **not** escaped. Non-ASCII is **not** rewritten as `\uXXXX`.

### 3.5 Literals

`true`, `false`, `null` as those four/five ASCII characters. Config
serialization does not emit `null`.

### 3.6 Insignificant whitespace

None. No space after `{`, `:`, `,`, or before `}`. No tabs. No
newlines inside the document. **No trailing newline.** No UTF-8 BOM.

Spaces that appear **inside string values** (for example an arm label)
are significant and are preserved.

---

## 4. `config_sha256`

1. Produce the canonical JSON string as in §3.
2. Interpret that string as UTF-8 bytes (it is already UTF-8; do not
   transcode; do not append a newline).
3. Compute SHA-256 of those bytes ([FIPS 180-4](https://csrc.nist.gov/publications/detail/fips/180/4/final)).
   This implementation uses the `sha2` crate (`sha2 = "=0.10.9"`), not a
   hand-rolled hash.
4. Encode the 32-byte digest as **64 lowercase hexadecimal characters**,
   no `0x` prefix, no separators.

That hex string is the `config_sha256` field written to manifests
(plan §6.2–6.3).

---

## 5. Worked example

The plan §5.1 config with `study_id` `DEMO-201` (also
`examples/stratified-block-variable.json`), object keys presented in
any order, canonicalizes to the following **single line** (shown
wrapped here only for the page):

```text
{"arms":[{"code":"A","label":"Investigational product 50 mg","ratio":2},{"code":"P","label":"Placebo","ratio":1}],"block":{"kind":"variable","sizes":[6,9]},"list_length_per_stratum":36,"method":"stratified_block","numbering":{"kind":"global","start":10001,"width":5},"protocol_version":"2.1","schema_version":"1.0","strata":[{"levels":["001","002","003"],"name":"site"},{"levels":["LT65","GE65"],"name":"agegrp"}],"study_id":"DEMO-201"}
```

Notes on that line:

- Top-level keys are ASCII-sorted: `arms`, `block`,
  `list_length_per_stratum`, `method`, `numbering`, `protocol_version`,
  `schema_version`, `strata`, `study_id`.
- Nested arm objects sort to `code`, `label`, `ratio`.
- Stratum objects sort to `levels`, `name` (not the struct field order
  `name`, `levels`).
- `sizes` remains `[6,9]` and site levels remain
  `["001","002","003"]` — arrays are not sorted.
- The label’s spaces are inside a string; they stay.

`config_sha256` of those bytes:

```text
1364cea5ed27222f7d53130d10dbe7d94eb70b79e5fe0e2d007d2aa9979f01be
```

The same logical config with every object’s keys reversed must produce
this exact string and this exact hex digest.

---

## 6. `list.csv`, `list.json`, and `stream.csv` (in-memory)

These renderers live in `clinrand-package` and are pure functions of
`GeneratedList` / `StreamLog` plus `StudyConfig` where needed. They do
**not** write the filesystem, do **not** include the seed, and do **not**
implement allocation.

Public API:

- `render_list_csv(&StudyConfig, &GeneratedList) -> Result<String, PackageError>`
- `render_list_json(&StudyConfig, &GeneratedList) -> Result<String, PackageError>`
- `render_stream_csv(&StreamLog) -> Result<String, PackageError>`

### 6.1 Shared encoding

- UTF-8 text, **LF** (`\n`) line endings, **no BOM**
- Each file ends with **exactly one trailing `\n`** after the last row
  (or after the header when there are no data rows). There is no extra
  blank line.
- CSV quoting is minimal RFC 4180-style: quote a field only if it
  contains comma, `"`, or a newline; escape `"` as `""`. Synthetic
  fixtures normally need no quotes.

SHA-256 of these files (when manifests hash them) is the digest of these
exact UTF-8 bytes, including the final newline.

### 6.2 `list.csv` (plan §6.1)

Header, then one row per `AllocationRecord` in list order:

```text
randomization_number,<one column per stratum factor>,block_id,block_size,position_in_block,arm_code
```

Stratum columns follow **config factor order** (`StudyConfig.strata`).
Do not sort factor names. Empty `strata` → no stratum columns between
`randomization_number` and `block_id`.

### 6.3 `list.json`

Structured equivalent of the same rows:

```text
{"records":[{...},{...}]}
```

plus a trailing `\n`. Each record object uses the **same keys as the
CSV columns**, emitted in that same order (config factor order for
stratum fields). Compact encoding: no insignificant whitespace. Keys are
not lexicographically sorted (unlike §6.4 canonical JSON), so that
stratum field order matches the CSV.

### 6.4 `stream.csv`

Header and columns:

```text
index,bound,value,purpose
```

One row per `StreamDraw` in log order. `purpose` is snake_case:

| `DrawPurpose`       | CSV value            |
|---------------------|----------------------|
| `BlockSize`         | `block_size`         |
| `Permutation`       | `permutation`        |
| `SimpleAllocation`  | `simple_allocation`  |

An empty stream is the header line plus trailing `\n` only.

---

## 7. Manifests (in-memory)

`build_manifests(&StudyConfig, &GeneratedList, &[u8; 32], &PackageMeta)`
returns `ManifestPair { unblinded, blinded }` — the UTF-8 contents of
`manifest.unblinded.json` and `manifest.blinded.json` (plan §6.2–§6.3).
It does not write the filesystem or read the clock; `PackageMeta`
supplies `operator` and `generated_at`.

### 7.1 Encoding

- Compact **canonical JSON** (same key-sort rules as §3 / plan §6.4)
- Exactly one trailing `\n` after the JSON object
- UTF-8, LF, no BOM

### 7.2 Fields

Shared by both manifests: `schema_version` (`"1.0"`), `study_id`,
`protocol_version`, `generated_at`, `operator`, `config` (full
`StudyConfig` wire object), `config_sha256`, `seed_sha256`, `rng`
(`algorithm` / `crate` / `crate_version`), `engine_version`
(`clinrand_core::ENGINE_VERSION` = core crate `CARGO_PKG_VERSION`),
`algo_version`, `record_count`, `list_sha256`, `stream_sha256`.
`rng.crate_version` is `clinrand_core::RNG_CRATE_VERSION` and must stay
in sync with the exact `rand_chacha = "=…"` pin in `clinrand-core`.

`generated_at` must be exactly `YYYY-MM-DDTHH:MM:SSZ` (`PackageMeta::new`
and `build_manifests` validate). The package crate does not read the clock.

Unblinded only: `seed_hex` (64 lowercase hex characters of the raw
32-byte seed). Blinded omits the `seed_hex` **key** entirely; it still
includes `seed_sha256`.

### 7.3 Content hashes

| Field | Digest input |
|---|---|
| `config_sha256` | Canonical JSON of `config` (§4); **no** trailing newline |
| `seed_sha256` | The **32 raw seed bytes** (not the hex string) |
| `list_sha256` | Exact UTF-8 bytes of `render_list_csv` (including final `\n`) |
| `stream_sha256` | Exact UTF-8 bytes of `render_stream_csv` (including final `\n`) |

All digests are SHA-256 encoded as 64 lowercase hex characters.

---

## 8. `write_package` and `checksums.txt`

`write_package(out_dir, cfg, list, seed, meta) -> Result<PathBuf, PackageError>`
creates a package directory and writes the full package file set,
including both HTML reports and `qc.R`. If the target package directory already
exists, `write_package` returns [`PackageError::PackageDirExists`] and
does not overwrite.

### 8.1 Directory name

```text
<out_dir>/<study_id>_<generated_at compact>_<first 8 hex of list_sha256>/
```

`generated_at` is taken from `PackageMeta` (caller-supplied; no clock
read). Compact form keeps ASCII alphanumerics only, so
`2026-09-15T14:42:10Z` becomes `20260915T144210Z`. An already-compact
value is left unchanged. `list_sha256` is SHA-256 of the exact
`list.csv` UTF-8 bytes (§6.2), lowercase hex; the directory uses the
first 8 characters.

### 8.2 Files written

| File | Source bytes |
|---|---|
| `list.csv` | `render_list_csv` |
| `list.json` | `render_list_json` |
| `stream.csv` | `render_stream_csv` |
| `manifest.unblinded.json` | `ManifestPair.unblinded` from `build_manifests` |
| `manifest.blinded.json` | `ManifestPair.blinded` from `build_manifests` |
| `generation-report.html` | `render_generation_report` |
| `unblinded-report.html` | `render_unblinded_report` |
| `qc.R` | `render_qc_r` |
| `checksums.txt` | See §8.3 |

Manifests are written as the exact `ManifestPair` strings — they are
**not** re-serialized. The seed appears on disk only in
`manifest.unblinded.json`.

### 8.3 `checksums.txt`

SHA-256 of every package file **except** `checksums.txt` itself
(including both HTML reports and `qc.R`). Format is GNU `sha256sum`
**text mode**:
one line per file

```text
<64 lowercase hex><two spaces><filename>\n
```

Lines are sorted by filename. Digests cover the exact UTF-8 bytes
written to each file (including trailing newlines as specified above).

---

## 9. HTML reports (plan §6.5)

Hand-rolled `format!` HTML in `clinrand-package` (no templating crate).

### 9.1 `generation-report.html` (blinded)

Must include: study id, protocol, `generated_at`, operator, full config
(arms+ratios, method, blocks, strata, numbering), record counts per
stratum, block structure (counts/sizes per stratum only — no arm
composition), `seed_sha256` and data-file hashes, engine/algo versions
(engine from `clinrand_core::ENGINE_VERSION`), [`check_properties`]
results (P01–P10 ids and pass/fail / informational), a static sentence
that **P10 is informational only and must not cause regeneration**, and
the P10 max-run detail when that detail contains no config arm-code
tokens. Other property-check details are omitted. Also truncation
warnings, and a `per_stratum_range` disclosure warning when that
numbering is used.

Per-stratum count and block-structure rows follow
`stratum_combinations` **canonical config order** (plan §5.4), including
combinations with count `0` / no blocks. Do not sort stratum presentation
by label. Numbering `start` / `width` (and `block_size`) are emitted on
separate HTML lines from `kind=` to avoid blind-safety false positives
with numeric arm codes.

Must **not** include: the seed (or `seed_hex`), any randomization-number
↔ arm pairing, per-block arm composition, or non-P10 property-check
detail text that could name both a randomization number and an arm.

Property results come from `clinrand_core::check_properties` — the
package crate does not reimplement P01–P10.

CI enforces blind-safety on a real `generate()`-derived DEMO list: for
every randomization number, no line of the rendered blinded HTML contains
both that number and any arm code (delimiter-aware tokens). The same
assertion run against `unblinded-report.html` must find at least one
violating line (negative control). The seed hex string must not appear.

E2E: `generate` twice with the same seed yields equal lists; writing both
packages under distinct parent temp dirs yields byte-identical `list.csv`
and `stream.csv`; `checksums.txt` verifies including both HTML files.

### 9.2 `unblinded-report.html` (restricted)

Same metadata sections, plus full property-check **details** and a full
allocation table (randomization number ↔ arm). Still never writes the
seed.

---

## 10. `restricted.age` (plan §6.6, `--encrypt`)

`write_package_encrypted(out_dir, cfg, list, seed, meta, passphrase) ->
Result<PathBuf, PackageError>` renders the same content as `write_package`
(§8) but writes only 4 files as plaintext — `generation-report.html`,
`manifest.blinded.json`, `qc.R`, and `restricted.age` — plus `checksums.txt`
covering those 4. `list.csv`, `list.json`, `manifest.unblinded.json`,
`stream.csv`, and `unblinded-report.html` never touch disk as plaintext.
`decrypt_package(package_dir, passphrase) -> Result<(), PackageError>`
reverses this **in place**: it writes those 5 files directly into
`package_dir`, refusing to overwrite any that already exist.

Design rationale, crate choice, and why this is a custom format rather than
the literal `age` file format: `docs/decisions/0007-restricted-container-format.md`.

### 10.1 Container layout

All multi-byte integers are little-endian.

| Field | Size | Contents |
|---|---|---|
| `magic` | 4 bytes | `b"CRV1"` — ClinRand restricted container, format v1 |
| `kdf_m_cost` | 4 bytes (`u32`) | Argon2id memory cost, KiB |
| `kdf_t_cost` | 4 bytes (`u32`) | Argon2id iteration count |
| `kdf_p_cost` | 4 bytes (`u32`) | Argon2id parallelism |
| `salt` | 16 bytes | Random, unique per encryption |
| `nonce` | 24 bytes | Random, unique per encryption (XChaCha20 nonce) |
| `ciphertext` | remainder | AEAD output — plaintext archive (§10.2) + 16-byte Poly1305 tag appended |

Header length is fixed at 56 bytes (`4+4+4+4+16+24`); everything after it is
ciphertext. New containers always use `m_cost=131072` (128 MiB), `t_cost=3`,
`p_cost=4`; existing containers with different values (a future retuned
default) remain decodable because the params travel in the header.

### 10.2 Plaintext archive (pre-encryption)

The AEAD plaintext is a minimal multi-file archive, no external dependency:
for each of the 5 restricted files, **sorted by filename**, concatenate:

```text
u16 name_len (LE)  |  name bytes (UTF-8, name_len bytes)
u64 content_len (LE)  |  content bytes (content_len bytes)
```

repeated once per file, with no separator or trailing marker — the reader
stops when it has consumed the whole plaintext. `content` bytes are exactly
the renderer output for that file (§6, §7, §9.2) — identical to what
`write_package` would have written as that file's plaintext bytes.

### 10.3 Key derivation and AEAD

- KDF: Argon2id (`argon2` crate), version `0x13`, output length 32 bytes.
  Input: the UTF-8 passphrase bytes and the container's 16-byte `salt`.
- AEAD: XChaCha20-Poly1305 (`chacha20poly1305` crate). Key: the 32-byte
  Argon2id output. Nonce: the container's 24-byte `nonce`.
- Associated data (authenticated, not encrypted):
  `"clinrand-restricted-v1:" + study_id + ":" + list_sha256`, where both
  values are read from the plaintext `manifest.blinded.json` (`study_id`
  and `list_sha256` top-level fields — both already public in the blinded
  manifest). This binds a container to its specific package: copying a
  `restricted.age` from one package directory into another fails AEAD
  authentication even with the correct passphrase.

### 10.4 `decrypt_package` verification order

1. Read `restricted.age`; compute its SHA-256 and compare against the
   `restricted.age` line in `checksums.txt`. Mismatch →
   `PackageError::ContainerCorrupt` (checksum tampering/corruption is
   distinguished from a wrong passphrase; AEAD is not attempted).
2. Read `study_id` / `list_sha256` from `manifest.blinded.json` and
   reconstruct the associated data (§10.3).
3. Derive the key from the passphrase and the container's `salt` /
   `kdf_*` fields.
4. AEAD-decrypt. Failure → `PackageError::DecryptionFailed` (wrong
   passphrase, or a container substituted from a different package) —
   the error never states which.
5. Parse the plaintext archive (§10.2) and write each file into
   `package_dir`, refusing (`PackageError::RestrictedFileExists`) if any
   target name already exists.

### 10.5 `checksums.txt` when encrypted

Same GNU `sha256sum` text-mode format as §8.3, but with 4 lines instead of
8: `generation-report.html`, `manifest.blinded.json`, `qc.R`,
`restricted.age`. `list.csv` etc. are not listed — their integrity is
covered transitively through `restricted.age`'s own checksum plus AEAD
authentication (§10.4), not a separate `checksums.txt` line. After
`decrypt_package`, `checksums.txt` is left unmodified — it stays a 4-line
file even though 5 more plaintext files now exist alongside it.
