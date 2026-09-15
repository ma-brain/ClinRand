# TODO

Phase 0 scaffold is in place. Remaining work follows
[`docs/plans/clinrand-implementation-plan.md`](docs/plans/clinrand-implementation-plan.md)
§12. Do not start a phase until the previous phase's "Done when"
criteria are met.

- [ ] Phase 1 — RNG layer (`ChaCha20Rng`, `uniform_below`, `StreamLog`, RFC 8439 / hand-worked reference cases, `docs/determinism.md`)
- [ ] Phase 2 — Config and canonicalization (`StudyConfig`, `validate_config`, JSON Schema, canonical JSON)
- [ ] Phase 3 — Generation engine (simple / permuted-block / stratified-block, numbering, P01–P10, proptest)
- [ ] Phase 4 — Package writer (`list.csv`, manifests, reports, checksums, blind-safety test)
- [ ] Phase 5 — CLI (`generate`, `reproduce`, `verify`, `validation-report`, exit codes)
- [ ] Phase 6 — Emitted `qc.R` and `docs/qc-procedure.md`
- [ ] Phase 7 — Desktop application (Tauri 2 + SvelteKit, capability lock-down, `docs/security-posture.md`)
- [ ] Phase 8 — Encryption at rest and tagged release installers
- [ ] Phase 9 — Validation completeness (`validation/README.md`, regression rules, handbook)
