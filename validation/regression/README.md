# Regression fixtures

**A broken fixture is never fixed by regenerating its expected hash.**

If a fixture under `algo-v1/` (or any later `algo-vN/`) no longer matches
this engine's output, that means exactly one of two things:

1. **The change under test is wrong.** Revert it or fix it so the engine's
   output matches the frozen fixture again.
2. **The determinism contract deliberately changed** (RNG, `uniform_below`,
   permutation, stream order, stratum order, or how many bytes are
   consumed — see `AGENTS.md §3`). In that case, increment
   `clinrand_core::ALGO_VERSION`, add a **new** `algo-v<N+1>/` directory
   with fresh fixtures frozen against the new behavior, and leave every
   existing `algo-vN/` directory untouched — old fixtures are never edited,
   deleted, or regenerated.

There is no third option. Editing an existing fixture's `expected` block to
make a test pass is exactly the failure mode this tier exists to catch —
doing it anyway defeats the entire purpose of a regression tier. See
`AGENTS.md §4` (hard prohibition 1) and `§9`.

## What this tier proves — and does not

Byte-identical output since the last approved `ALGO_VERSION`. **Consistency,
not correctness.** A fixture matching only proves nothing has silently
changed; it says nothing about whether the algorithm was right in the first
place. Correctness evidence lives in `validation/reference/` (external
normative sources); invariant evidence lives in `validation/properties/`.
See `../README.md` for the full three-tier distinction.

## Fixture format

One JSON file per case, directly inside an `algo-vN/` directory:

```json
{
  "case_id": "algo-v1-simple-global",
  "seed_hex": "<64 lowercase hex characters>",
  "config": { "...": "a full StudyConfig, same shape as examples/*.json" },
  "expected": {
    "config_sha256": "<64 lowercase hex>",
    "list_sha256": "<64 lowercase hex>",
    "stream_sha256": "<64 lowercase hex>",
    "record_count": 24
  }
}
```

- `config_sha256` — SHA-256 of the canonical JSON of `config` (plan §6.4,
  `docs/output-package.md` §3–4).
- `list_sha256` / `stream_sha256` — SHA-256 of the exact `render_list_csv` /
  `render_stream_csv` UTF-8 bytes for `(config, seed)` (`docs/output-package.md`
  §6), including trailing newlines.
- `record_count` — `GeneratedList::records.len()`.

Loading and checking is implemented once, in `clinrand-package`
(`crates/clinrand-package/src/regression.rs`: `load_regression_cases` /
`check_regression_case`) — not reimplemented separately by the CLI, the
desktop app, or the `cargo test` below, so there is exactly one place that
can be wrong about what "matches" means.

## `seed_hex` is synthetic, not secret

Every fixture's `seed_hex` is a hardcoded, arbitrary, made-up value with no
real study behind it — the same synthetic-data rule that already applies to
every `config` in this tier and in `examples/`
(`AGENTS.md §4` hard prohibition 10: "Configs, lists, seeds, and fixtures
are synthetic"). This is not a contradiction of `AGENTS.md §4.9` ("never
write the seed... to a log line... or commit it"): that rule protects a
**real** trial's production seed, which is equivalent to the list for an
actual study. A fixture's seed exists purely to pin a deterministic test
and unlocks nothing about any real allocation.

## Adding a fixture to the current `algo-v1/` set

New cases for the **same** `ALGO_VERSION` may be added freely — that's not
"changing" a fixture, it's adding coverage. To add one:

1. Pick a synthetic config (reuse one of `examples/*.json`, or write a new
   one with a `DEMO-`/`TEST-`/`EXAMPLE-` study ID).
2. Pick an arbitrary, hardcoded 32-byte seed (any value — it only needs to
   be fixed, not secret or random).
3. Run the current engine against `(config, seed)` — do not hand-compute or
   guess the expected hashes — and record the real
   `config_sha256` / `list_sha256` / `stream_sha256` / `record_count`.
4. Write the fixture JSON and confirm `cargo test -p clinrand-package
   --test regression_fixtures` passes.

## Where this runs

- `cargo test -p clinrand-package --test regression_fixtures` — automated,
  runs in `cargo test --workspace` / CI, per `AGENTS.md §8`'s non-negotiable
  "every regression fixture's `list_sha256` matches" requirement.
- `clinrand validation-report --tier regression` — human-readable report,
  same underlying check.
- The desktop app's Validation screen (`run_validation_report` Tauri
  command) — same check again, adapted in-process per the Phase 7 ruling
  documented in `apps/desktop/src-tauri/src/commands/validation.rs`.
