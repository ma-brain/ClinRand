# Property validation tier

This tier asserts **invariants** of generated lists (plan §7 / §8.2), not
correctness against an external oracle.

- Cases are produced by `proptest` over arbitrary **valid** configs.
- Each case runs `generate` then `check_properties` (P01–P09 required).
- A passing property suite means the engine preserved its stated invariants;
  it does **not** prove the algorithm matches an independent reference
  implementation.

Expected values are not stored here. Reference evidence lives under
`validation/reference/`; frozen byte-identity fixtures live under
`validation/regression/` (when present).

The executable suite is `crates/clinrand-core/tests/properties_proptest.rs`
(minimum **1000** cases in CI).
