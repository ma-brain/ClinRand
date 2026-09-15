# QC procedure

Every package ClinRand writes includes a study-specific `qc.R` script.
An unblinded statistician runs it as part of independent QC before a list
is approved for trial use. This document describes how to run the script,
what PASS and FAIL mean, and how to handle failures.

The script is emitted in every package directory. It is not a substitute
for formal approval: responsibility for the list rests with the statistician
who signs off, not with the software.

## Prerequisites

- **R** (base R is sufficient; no tidyverse or other add-on collections)
- **`jsonlite`** and **`digest`** only — no other CRAN packages

Install the two packages once on the QC machine:

```r
install.packages(c("jsonlite", "digest"))
```

CI and the integration tests use the same dependency set. Do not add packages
to `qc.R`; a locked-down validated R installation must be able to run it.

## How to run

1. Open a terminal and **change into the package directory** — the folder
   that contains `manifest.unblinded.json`, `list.csv`, `stream.csv`, and
   `qc.R`.
2. Run:

   ```bash
   Rscript qc.R
   ```

The script reads files from the **current working directory** using relative
paths. It takes no arguments. Do not run it from a parent directory.

On success the process exits `0` and prints `OVERALL: PASS`. On any required
failure it exits non-zero and prints `OVERALL: FAIL`.

## What the script checks

`qc.R` implements plan §9.1. In order it:

1. Reads `manifest.unblinded.json`, `list.csv`, and `stream.csv`.
2. Re-derives the canonical stratum order from the config (independent of
   column order in `list.csv`).
3. Rebuilds every block from `stream.csv` by reimplementing block-size
   selection and Fisher–Yates in R, consuming the recorded draws in order.
   It does **not** reimplement ChaCha20; the RNG is verified separately
   against RFC 8439 in `validation/reference/`.
4. Compares the reconstructed allocation row-for-row against `list.csv`.
5. Recomputes SHA-256 of `list.csv` and `stream.csv` and compares against
   the manifest.
6. Recomputes `config_sha256` from the manifest config using the same
   canonical JSON rules as the engine (`docs/output-package.md`).
7. Re-runs property checks **P01–P09** (same semantics as
   `clinrand_core::check_properties`; P10 is informational only and is not
   evaluated as a pass/fail gate in `qc.R`).

Each step prints a line of the form `[PASS] id: detail` or
`[FAIL] id: detail`. The script ends with `OVERALL: PASS` or
`OVERALL: FAIL` and `sessionInfo()`.

### Property checks (P01–P09)

| ID | What it verifies |
|----|------------------|
| P01 | Record count equals `list_length_per_stratum ×` number of stratum combinations (truncation allowed) |
| P02 | Every block size is in the configured allowed set |
| P03 | Every complete block contains the correct arm counts for the allocation ratio |
| P04 | No duplicate randomization numbers |
| P05 | Randomization numbers are contiguous and ascending within their scheme |
| P06 | Every stratum combination in the Cartesian product is present |
| P07 | No stratum combination contains records from another stratum |
| P08 | `position_in_block` is 1..block_size, complete and without gaps, per block |
| P09 | Overall arm counts match the allocation ratio within one block's tolerance |

See plan §7 for full definitions. P10 (maximum run length) may appear in
HTML reports as informational only; it must not cause regeneration.

## Interpreting PASS and FAIL

**PASS** means every required check succeeded: stream reconstruction matches
`list.csv`, file hashes match the manifest, `config_sha256` matches, and
P01–P09 all passed. The list is internally consistent with the recorded
stream and configuration.

**FAIL** means at least one required check failed. Read the `[FAIL]` lines
to see which step broke. Common causes:

- **Reconstruction mismatch** — `list.csv` does not follow from
  `stream.csv` under the documented blocking and permutation rules (possible
  manual edit, wrong file, or corruption).
- **Hash mismatch** — `list.csv` or `stream.csv` bytes differ from what the
  manifest recorded.
- **Property failure** — the list violates P01–P09 (e.g. duplicate
  randomization numbers, wrong block structure).

A FAIL result is evidence to investigate. It is not an instruction to
regenerate.

## If QC fails — do not regenerate to "fix" it

**Never regenerate a list to make a QC failure go away.**

If `qc.R` or `clinrand verify` reports FAIL:

1. **Stop.** Do not approve the list.
2. **Preserve** the package directory unchanged (including `stream.csv` and
   both manifests).
3. **Investigate** — determine whether the failure is corruption, a wrong
   file, a manual edit, or a genuine engine defect.
4. **Document** findings in the study QC record.

Regenerating with a new seed produces a **different** list. It does not
prove the original list was correct. Treating regeneration as a fix destroys
auditability and can hide a serious error.

If you believe the engine is wrong, open an issue with the failing package
(redact `seed_hex` from the unblinded manifest before sharing). Do not
change regression fixture hashes or bump `ALGO_VERSION` without following
`AGENTS.md` and the validation tier rules.

## Seed handling

The seed is equivalent to the list. Anyone with `seed_hex` from
`manifest.unblinded.json` can reproduce the allocation.

- **`qc.R` never prints the seed.** It does not read or log `seed_hex`.
- Treat **`manifest.unblinded.json`** like the list: store it securely,
  restrict access to unblinded staff, and do not commit it to version
  control or paste it into tickets.
- The blinded manifest omits `seed_hex` by design. The blinded report
  never pairs randomization numbers with arms.

For reproduction from a saved manifest, use `clinrand reproduce` (see
`docs/cli.md`). That command also refuses to run if the manifest's
`algo_version` differs from the binary.

## Complementary checks

### `clinrand verify`

The CLI command `clinrand verify --package <dir>` recomputes every hash in
`checksums.txt` (including `qc.R`) and re-runs property checks against
`list.csv` without regenerating. Use it for a quick integrity pass; use
`qc.R` for the full independent reconstruction from `stream.csv`.

### Optional: distributional comparison with `blockrand` or `randomizeR`

Plan §9.3 describes an **optional** supplementary path, not emitted by
ClinRand:

1. Using the same study design (arms, ratios, block sizes, strata), generate
   many lists with **`blockrand`** or **`randomizeR`** in R.
2. Compare **distributional properties only** — for example arm balance,
   block-size frequency, permutation patterns over replicates.

**This path cannot produce an identical sequence to ClinRand's list.** Different
RNGs, stream consumption, and implementation details mean byte-for-byte or
row-for-row agreement is impossible. Do not present a `blockrand`/`randomizeR`
run as confirmation that a specific ClinRand list is "the same list."

Use distributional comparison only as additional comfort that the design
parameters behave as expected — never as a substitute for `qc.R` PASS or
formal approval.

## Related documentation

- Package layout and checksums: [`docs/output-package.md`](output-package.md)
- CLI (`generate`, `verify`, `reproduce`): [`docs/cli.md`](cli.md)
- Determinism contract (Fisher–Yates, stream order): [`docs/determinism.md`](determinism.md)
- Validation tiers: [`validation/README.md`](../validation/README.md)
