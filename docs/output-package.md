# Output package — canonical JSON, hashing, and list/stream renderers

This document specifies how ClinRand produces **canonical JSON**,
**`config_sha256`**, and the in-memory **`list.csv` / `list.json` /
`stream.csv`** byte layouts. Canonical JSON is precise enough for an
independent reimplementation of the config hash. Manifests, filesystem
writes, and reports are defined in
[`docs/plans/clinrand-implementation-plan.md`](plans/clinrand-implementation-plan.md)
§6 and will be documented here as those pieces land.

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
(workspace package version), `algo_version`, `record_count`,
`list_sha256`, `stream_sha256`.

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
