# Determinism contract

This document is the normative specification of the ClinRand allocation-path
randomness layer. It restates
[`docs/plans/clinrand-implementation-plan.md`](plans/clinrand-implementation-plan.md)
§2.1–2.6 in enough detail that a third party can reimplement the RNG layer
without reading the Rust sources.

A given `(config, seed)` pair must produce a byte-identical list on every
platform, on every run, forever — until `ALGO_VERSION` is deliberately
incremented. Anything in this document is contract-bound.

---

## 1. Random number generator (plan §2.1)

### 1.1 Type and seed

- Use `rand_chacha::ChaCha20Rng` only.
- Pin the crate with an exact version requirement: `rand_chacha = "=0.3.1"`.
  `rand_core = "=0.6.4"` is the companion pin required by that release.
  Commit `Cargo.lock`.
- Production construction seeds the generator from a **256-bit key** (`[u8;
  32]`). Stream id and word position start at **0** (`ChaCha20Rng::from_seed`).
- `getrandom` is used in **one place only**: drawing a fresh 256-bit seed at
  the start of a generation run. The allocation path inside `clinrand-core`
  never calls it.

### 1.2 Forbidden sources

Do **not** use `rand::thread_rng`, `StdRng`, `SmallRng`, `rand::random`,
`OsRng`, or any RNG sourced from the environment, the clock, or thread state in
the allocation path.

### 1.3 Byte interface

Production code consumes randomness through two methods on the engine's
`Rng` wrapper:

- `fill_bytes(dest)` — fill `dest` with the next keystream bytes.
- `next_u64()` — return the next 64 bits from the stream.

`uniform_below` (§2) draws exclusively through `next_u64`. No other distribution
helpers from the `rand` crate may appear in the allocation path.

### 1.4 IETF positioning (reference tests only)

[RFC 8439](https://www.rfc-editor.org/rfc/rfc8439.html) (*ChaCha20 and
Poly1305 for IETF Protocols*, Nir & Langley, June 2018) publishes ChaCha20
test vectors with a 96-bit nonce and 32-bit block counter. `rand_chacha`
0.3.1 uses Bernstein's 64-bit counter and 64-bit stream id instead. The
engine exposes a test-only `from_ietf(key, nonce, block_counter)` that maps
IETF nonce/counter into that representation so the same `ChaCha20Rng` type can
be checked against RFC §2.3.2 and §2.4.2. **The allocation path does not call
`from_ietf`.**

Normative expected bytes live under
[`validation/reference/chacha20/`](../validation/reference/chacha20/). See
[`reference-output.md`](../validation/reference/chacha20/reference-output.md)
for the RFC citations and case derivations.

---

## 2. Uniform integer draws (plan §2.2)

Implement `uniform_below(rng, log, n, purpose) -> Result<u64, UniformError>`
using **rejection sampling**. Do not substitute `Rng::gen_range` or any other
`rand` distribution — their internals are not stable across `rand` versions.

### 2.1 Algorithm

1. If `n == 0`, return an error (`ZeroBound`). Consume **zero** `next_u64`
   words.
2. If `n == 1`, return `0`. Consume **zero** `next_u64` words. Do **not**
   append to the stream log (§3).
3. Otherwise:
   - Compute `remainder = u64::MAX % n`.
   - Compute `zone = u64::MAX - remainder` (equivalently
     `u64::MAX.saturating_sub(remainder)`).
   - Loop:
     - Draw `x = rng.next_u64()`.
     - If `x >= zone`, discard `x` and redraw.
     - Otherwise let `value = x % n`, record the accepted draw in the stream
       log (§3), and return `value`.

`u64::MAX` is `18_446_744_073_709_551_615`.

### 2.2 Why rejection sampling

Let `zone = u64::MAX - (u64::MAX % n)`. Then `zone` is the largest multiple of
`n` not exceeding `u64::MAX`, so every integer in `0..zone` is equally likely
to be congruent to each residue mod `n`. Rejecting `x >= zone` removes the
short remainder bucket and yields a uniform draw in `0..n`.

### 2.3 Reference cases

Hand-worked cases with a documented integer keystream (not ChaCha20 output) are
under [`validation/reference/uniform-below/`](../validation/reference/uniform-below/).
See
[`reference-output.md`](../validation/reference/uniform-below/reference-output.md)
for step-by-step derivations, including:

- `n == 0` — error, zero consumption.
- `n == 1` — returns `0`, zero consumption.
- `n == 3` — acceptance with no rejection (`zone == u64::MAX`).
- `n == 2` — one rejection then acceptance (`zone == u64::MAX - 1`).

---

## 3. Stream log (plan §4)

`StreamLog` records **accepted** `uniform_below` draws only. Each entry
(`StreamDraw`) carries:

| Field | Meaning |
|---|---|
| `index` | Zero-based position in `draws` |
| `bound` | The `n` passed to `uniform_below` |
| `value` | The returned `x % n` |
| `purpose` | Caller label: `BlockSize`, `Permutation`, or `SimpleAllocation` |

Rejected samples (`x >= zone`) still consume keystream words via
`next_u64`, but are **not** logged. `n == 1` produces no log entry.

---

## 4. Permutation (plan §2.3)

Block contents are shuffled with **Fisher–Yates**, descending index:

```
for i in (1..len).rev():
    j = uniform_below(rng, log, i + 1, Permutation)
    swap(items[i], items[j])
```

`j` is always in `0..(i + 1)` inclusive of `i`. The descending loop order is
part of the contract. Length 0 and length 1 perform no draws.

Implemented as `clinrand_core::permute`. Hand-worked reference cases live
under [`validation/reference/fisher-yates/`](../validation/reference/fisher-yates/).

---

## 5. Stream consumption order (plan §2.4)

The order in which the engine consumes randomness is part of the contract. One
`ChaCha20Rng` instance is created per **run** and shared across all strata.
Strata are processed sequentially in **canonical stratum order** (§6).

For each stratum, and for each block in ascending block index:

1. **Block size** (variable blocks only): draw
   `uniform_below(rng, log, sizes.len(), BlockSize)` and use the result to
   index into the sorted, deduplicated `sizes` array.
2. Build the block's arm multiset from the integer allocation ratios.
3. **Permute** the multiset with Fisher–Yates (§4).

Fixed block size skips step 1.

### Simple randomization

For `method: simple`, there is no block-size draw and no Fisher–Yates.
For each stratum in canonical order, and for each position in
`0..list_length_per_stratum`:

1. Draw `uniform_below(rng, log, ratio_sum, SimpleAllocation)`.
2. Map the draw to an arm via cumulative ratios in **config arm order**
   (the first arm owns `[0, ratio)`, the next owns the following
   `ratio` integers, and so on).

Each position is recorded as a size-1 block (`block_id` ascending from
1 within the stratum, `block_size = 1`, `position_in_block = 1`).

---

## 6. Canonical stratum order (plan §5.4)

Factors appear in the order given in the config array — **do not sort them**.
Levels appear in the order given in each factor's array. The Cartesian product
is enumerated with the **last factor varying fastest**.

This ordering is part of the determinism contract. Sorting strata for
convenience would change which random draws belong to which stratum.

---

## 7. No floating point (plan §2.5)

The allocation path contains no `f32` or `f64`. Allocation ratios, block sizes,
and counts are integers. Floating-point values may appear only in presentational
summaries in reports, never in anything that influences allocation, stream
consumption, or hashing.

---

## 8. Version identity (plan §2.6)

Every generated package records two version fields:

| Field | Meaning |
|---|---|
| `engine_version` | Crate version of `clinrand-core` (e.g. `0.1.0`) |
| `algo_version` | Plain integer (`ALGO_VERSION` in `clinrand-core`; currently `1`) |

`ALGO_VERSION` is **not** the crate version. Increment it whenever anything in
§1–§7 of this document changes, including changes that only alter how many
bytes are consumed or the order of consumption. Do **not** increment it for
report formatting, CLI flags, UI work, or documentation edits.

`clinrand reproduce` refuses outright when a manifest's `algo_version` differs
from the binary's. There is no bypass flag.

---

## 9. Validation references

| Tier | Path | What it proves |
|---|---|---|
| Reference | [`validation/reference/chacha20/`](../validation/reference/chacha20/) | ChaCha20 keystream matches [RFC 8439](https://www.rfc-editor.org/rfc/rfc8439.html) §2.3.2 and §2.4.2 |
| Reference | [`validation/reference/uniform-below/`](../validation/reference/uniform-below/) | `uniform_below` matches hand-worked rejection-sampling arithmetic |
| Reference | [`validation/reference/fisher-yates/`](../validation/reference/fisher-yates/) | Descending Fisher–Yates matches hand-worked permutations over a documented keystream |

These tiers carry **correctness** evidence from external or hand-derived
norms. They are not regression fixtures; expected values were not taken from
this engine's own prior output.

---

## 10. Change control

Any change to §1–§7 requires:

1. Increment `ALGO_VERSION`.
2. A dedicated commit with a prominent `CHANGELOG.md` entry.
3. A new regression fixture set under `validation/regression/algo-vN/`.

When unsure whether a change is contract-bound, stop and ask. A wrong list is
indistinguishable from a correct one by inspection.
