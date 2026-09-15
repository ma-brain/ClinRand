# ClinRand CLI

The `clinrand` binary is the command-line interface to the same engine and
package writer used by the desktop application. All allocation and hashing
logic lives in `clinrand-core` and `clinrand-package`; the CLI only handles
I/O, exit codes, and output formatting.

Run commands via the project wrapper or directly:

```bash
just cli -- generate --config examples/simple.json --out /tmp/out --operator "Jane Statistician"
cargo run -p clinrand-cli -- generate --config examples/simple.json --out /tmp/out --operator "Jane Statistician"
```

After a release build, the binary is `target/release/clinrand`.

## Global options

| Option | Description |
|--------|-------------|
| `--json` | Emit machine-readable JSON on stdout. Applies to every subcommand. Never includes `seed_hex` or raw seed material. |

## Exit codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | Check or verification failure (hash mismatch, property check failure, reproduce list mismatch) |
| `2` | Invalid configuration (`validate-config` errors, or `generate` rejected config) |
| `3` | I/O or parse failure (missing file, malformed JSON, unreadable package path) |
| `4` | Manifest `algo_version` differs from the binary's `ALGO_VERSION` (`reproduce` only) |

There is no bypass for exit code 4. If a manifest records a different
algorithm version than the running binary, reproduction is refused outright.

## Commands

### `list-methods`

Print supported randomization methods.

```bash
just cli -- list-methods
just cli -- --json list-methods
```

Human output: one method per line (`simple`, `permuted_block`, `stratified_block`).

JSON output: array of method name strings.

Exit `0` on success.

### `validate-config`

Validate a study configuration file without generating a list.

```bash
just cli -- validate-config --config examples/simple.json
just cli -- --json validate-config --config examples/simple.json
```

| Option | Description |
|--------|-------------|
| `--config <file>` | Path to study configuration JSON (required) |
| `--allow-large-strata` | Allow more than 200 stratum combinations |

Exit `0` when valid (warnings may appear on stderr). Exit `2` when validation
errors are present. Exit `3` when the file cannot be read or parsed.

JSON output includes `ok`, `errors`, and `warnings` arrays.

### `generate`

Draw a fresh 256-bit seed, allocate a list, and write a full output package.

```bash
just cli -- generate \
  --config examples/simple.json \
  --out /tmp/clinrand-out \
  --operator "Jane Statistician"
```

| Option | Description |
|--------|-------------|
| `--config <file>` | Path to study configuration JSON (required) |
| `--out <dir>` | Parent directory for the package (required) |
| `--operator <name>` | Operator name recorded in manifests (required) |
| `--allow-large-strata` | Allow more than 200 stratum combinations |

**Note:** `--encrypt` is specified in the implementation plan but deferred to
Phase 8. It is not available in Phase 5.

The command writes a package subdirectory under `--out` and prints the package
path on stdout (line 1) and `list_sha256` (line 2). The seed is never printed
to stdout or stderr.

Exit `0` on success. Exit `2` for invalid config. Exit `3` for I/O errors
(missing config, unwritable output parent, etc.).

JSON output includes `package_dir`, `list_sha256`, and `record_count`. It does
not include `seed_hex`.

When `per_stratum_range` numbering is used, a disclosure warning is printed to
stderr and recorded in the package reports.

### `reproduce`

Regenerate a list from `manifest.unblinded.json` and assert the new
`list_sha256` matches the manifest.

```bash
just cli -- reproduce \
  --manifest /path/to/pkg/manifest.unblinded.json \
  --out /tmp/repro-out
```

| Option | Description |
|--------|-------------|
| `--manifest <file>` | Path to unblinded manifest (required) |
| `--out <dir>` | Parent directory for the reproduced package (required) |

Exit `0` when reproduction succeeds and hashes match. Exit `1` when the
regenerated list hash differs from the manifest. Exit `3` for I/O or manifest
parse errors. Exit `4` when `algo_version` in the manifest does not match the
binary — no package is written in that case.

JSON output includes `package_dir` and `list_sha256`.

### `verify`

Recompute every hash in `checksums.txt` and re-run property checks against
`list.csv` without regenerating.

```bash
just cli -- verify --package /path/to/pkg
just cli -- --json verify --package /path/to/pkg
```

| Option | Description |
|--------|-------------|
| `--package <dir>` | Path to the package directory (required) |

Exit `0` when checksums and properties pass. Exit `1` on checksum or property
failure. Exit `3` when the package path is missing or not a directory.

Human success output: `verify ok`. JSON output includes `ok`, `checksums_ok`,
`properties_ok`, and failure detail arrays.

### `validation-report`

Run validation-tier checks and emit a report to stdout.

```bash
just cli -- validation-report
just cli -- validation-report --tier reference --format md
just cli -- validation-report --tier properties --format html
just cli -- validation-report --tier regression --format md
just cli -- validation-report --tier all --format md
```

| Option | Description |
|--------|-------------|
| `--tier <name>` | `reference`, `properties`, `regression`, or `all` (default: all tiers) |
| `--format <fmt>` | `md` or `html` (default: `md`) |

Exit `0` when the report completes. Exit `1` when a tier reports failures.
Exit `3` on I/O errors reading validation fixtures.

### `version`

Print engine and algorithm version information.

```bash
just cli -- version
just cli -- --json version
```

Human output includes `engine_version`, `rng_crate_version`, and
`algo_version`. JSON output includes the same fields as structured data.

Exit `0` on success.

## Typical workflow

```bash
# 1. Validate configuration
just cli -- validate-config --config examples/simple.json

# 2. Generate a package (seed drawn from OS entropy — not logged)
just cli -- generate \
  --config examples/simple.json \
  --out ~/clinrand-packages \
  --operator "Jane Statistician"

# 3. Verify the package
just cli -- verify --package ~/clinrand-packages/DEMO-SIMPLE-1_.../

# 4. Reproduce from the unblinded manifest (QC / inspection)
just cli -- reproduce \
  --manifest ~/clinrand-packages/DEMO-SIMPLE-1_.../manifest.unblinded.json \
  --out ~/clinrand-repro
```

The unblinded manifest contains `seed_hex` and must be handled as sensitive
material. The CLI never echoes it to the terminal.
