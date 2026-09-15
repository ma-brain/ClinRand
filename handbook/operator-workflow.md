# Operator workflow: a complete study from config to approved package

This is one continuous walkthrough of everything between "I have a study
design" and "this list is approved and archived." It uses
`examples/stratified-block-variable.json` (study `DEMO-201`, a two-factor
stratified design) as the running example — the same config used as the
worked example in `docs/output-package.md`. Every command shown runs as
written from the repository root.

Commands are shown via the CLI (`just cli ...`); the desktop app does
the same steps through its GUI screens, noted inline. Either is a complete,
valid workflow — pick whichever suits the operator.

## 0. Before you start

- **This tool produces no approved artifact on its own.** Independent QC
  (§4 below) and formal sign-off (§5) are mandatory before any use in a
  trial — see the scope statement in the project [`README.md`](../README.md).
- Use synthetic study IDs (`DEMO-`, `TEST-`, `EXAMPLE-`) for anything that
  isn't a real study — never commit real study data, configs, or seeds
  (`AGENTS.md §4`, hard prohibition 10). This handbook only ever uses
  `DEMO-201`.
- Decide **now** whether restricted files will be encrypted at rest
  (§2 below). Deciding after generation just means running `decrypt` once
  before proceeding — it isn't a one-way door — but knowing the
  passphrase policy for the study up front avoids improvising it later.

## 1. Write and validate the config

Write a `StudyConfig` JSON file (`docs/plans/clinrand-implementation-plan.md`
§5.1 / `docs/schema/study-config-1.0.json`). Validate it before touching
generation:

```bash
just cli validate-config --config examples/stratified-block-variable.json
```

Exit `0` means the config is valid (non-fatal warnings, if any, print to
stderr — e.g. a `per_stratum_range` disclosure warning). Exit `2` means fix
the listed errors and re-run. **Desktop equivalent:** the Config builder
screen validates live as you type; it must clear to "valid" before
Generate is reachable.

## 2. Generate

```bash
just cli generate \
  --config examples/stratified-block-variable.json \
  --out ~/clinrand-packages \
  --operator "Jane Statistician"
```

This draws a fresh 256-bit seed from OS entropy (never printed, never
logged), allocates the list, and writes a package directory under
`--out`. Stdout prints the package path (line 1) and `list_sha256`
(line 2).

**Encrypting restricted files at rest:** add `--encrypt` and enter a
passphrase twice when prompted. `list.csv`, `list.json`,
`manifest.unblinded.json`, `stream.csv`, and `unblinded-report.html` are
then wrapped into a single `restricted.age` container instead of written
as plaintext — nothing else about this workflow changes, except that §4
and §5 below require running `clinrand decrypt --package <dir>` first (one
passphrase prompt). See `docs/decisions/0007-restricted-container-format.md`
and `0008-cli-decrypt-command-and-passphrase-ux.md` for why, and
`docs/cli.md` for the exact flags and exit codes.

**Desktop equivalent:** the Generate screen — operator name, destination
folder via the OS picker, an explicit confirmation that the output contains
unblinded material, and an optional "Encrypt restricted files" checkbox
with passphrase + confirmation fields.

## 3. Review the blinded material first

Before touching anything restricted, open `generation-report.html` from the
package directory. It contains the full config (arms, ratios, method,
block scheme, strata, numbering), record counts per stratum, block
structure, file hashes, and the property-check results (P01–P10) — **and
never a randomization-number-to-arm pairing** (enforced by an automated
test: `crates/clinrand-package/tests/blind_safety.rs`). This is safe to
share with blinded study staff.

**Desktop equivalent:** the Package viewer's inline blinded report, plus
`clinrand verify --package <dir>` (or the viewer's Verify button) to
re-check every `checksums.txt` hash and re-run properties against
`list.csv` without regenerating.

## 4. Independent QC

Every package includes a study-specific `qc.R` that independently
reconstructs the allocation from `stream.csv` and cross-checks every hash
and property. Full procedure, PASS/FAIL interpretation, and what to do on
a FAIL: **[`docs/qc-procedure.md`](../docs/qc-procedure.md)** — this
handbook doesn't repeat it. Summary:

```bash
cd ~/clinrand-packages/DEMO-201_.../   # the package directory itself
Rscript qc.R
```

If the package was `--encrypt`ed, decrypt in place first (one passphrase
prompt) — `qc.R` then finds `list.csv` / `stream.csv` /
`manifest.unblinded.json` exactly where it expects them:

```bash
just cli decrypt --package ~/clinrand-packages/DEMO-201_.../
```

`OVERALL: PASS` means the list is internally consistent with its recorded
stream and configuration. **A FAIL is evidence to investigate, never an
instruction to regenerate** — see `docs/qc-procedure.md`'s "if QC fails" §.

## 5. Formal approval

Nothing in ClinRand approves a list. This is the record a qualified
unblinded statistician should compile and sign before the list is used:

- [ ] Package directory path and `list_sha256` (printed by `generate`, also
      in every manifest).
- [ ] `qc.R` output: the full `[PASS]`/`[FAIL]` transcript and
      `OVERALL: PASS`.
- [ ] `clinrand verify --package <dir>` output (checksums + properties
      PASS; for a package decrypted from `restricted.age`, confirm
      `properties_checked: true` — not the "properties not checked, still
      encrypted" state).
- [ ] `clinrand validation-report --tier all` output for the ClinRand
      version used, or a reference to CI evidence for that version —
      establishes the *engine* was validated, distinct from validating
      *this list*.
- [ ] Reviewer name, role, and date.
- [ ] Any deviation investigated during QC and its resolution, even if the
      final result was PASS.

Retain this record with the package. **Responsibility for the list rests
with the statistician who signs this, not with the software** — restated
here in operational terms; see the project [`README.md`](../README.md)
scope statement for the principle.

## 6. Archival

Store the package directory (and the approval record from §5) somewhere
access-controlled and durable for the life of the trial plus whatever
retention period applies. If encryption at rest is part of the study's
data-handling policy, `--encrypt` (§2) or `clinrand decrypt`'s inverse (run
`generate --encrypt` in the first place, or re-encrypt manually before
archiving — ClinRand has no "encrypt an existing plaintext package"
shortcut beyond generating encrypted from the start) keeps the restricted
files at rest behind a passphrase; `manifest.blinded.json` and
`generation-report.html` stay plaintext regardless, since they're
never restricted.

Years later, an inspector may ask the statistician to prove the approved
list matches what ClinRand produced. `clinrand reproduce --manifest
manifest.unblinded.json --out <dir>` regenerates the list from the saved
unblinded manifest and asserts the new `list_sha256` matches — refusing
outright (exit `4`, no bypass) if the manifest's `algo_version` no longer
matches the binary's. This is the reason `manifest.unblinded.json` and the
seed it contains must survive archival alongside `list.csv`, not be
discarded as "redundant" once the list looks correct.
