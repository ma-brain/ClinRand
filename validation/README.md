# Validation tiers

ClinRand uses three validation tiers with different evidential meaning. Do not
conflate them.

| Tier | Directory | What it proves |
|---|---|---|
| **Reference** | `validation/reference/` | Correctness against external normative sources (RFC 8439, hand-worked derivations). |
| **Properties** | `validation/properties/` | Invariants hold for arbitrary valid configs — not external-oracle correctness. |
| **Regression** | `validation/regression/` | Byte-identical output since the last approved `ALGO_VERSION` — consistency, not correctness. |

Run `clinrand validation-report` for a human-readable summary (`--tier all`,
or `reference` / `properties` / `regression` individually; `--format md` or
`html`). All three tiers render real results — none is a permanent skip.
Full property coverage is `cargo test -p clinrand-core --test
properties_proptest` (1000 cases in CI); full regression coverage is
`cargo test -p clinrand-package --test regression_fixtures`.

Regression fixtures live under `validation/regression/algo-v1/`, one per
`ALGO_VERSION`. **Never regenerate a fixture's expected hash to make a test
pass** — see `validation/regression/README.md` for the full rule and the
fixture format.

For a complete operator walkthrough from config to approved package, see
[`handbook/`](../handbook/README.md).
