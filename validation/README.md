# Validation tiers

ClinRand uses three validation tiers with different evidential meaning. Do not
conflate them.

| Tier | Directory | What it proves |
|---|---|---|
| **Reference** | `validation/reference/` | Correctness against external normative sources (RFC 8439, hand-worked derivations). |
| **Properties** | `validation/properties/` | Invariants hold for arbitrary valid configs — not external-oracle correctness. |
| **Regression** | `validation/regression/` | Byte-identical output since the last approved `ALGO_VERSION` — consistency, not correctness. |

Run `clinrand validation-report` for a human-readable summary. Full property
coverage is `cargo test -p clinrand-core --test properties_proptest` (1000 cases
in CI). Phase 9 expands this documentation and regression fixtures.
