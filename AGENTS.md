# AGENTS.md — ClinRand

Operating rules for any coding agent working in this repository. Read this file completely before making any change. `docs/plans/clinrand-implementation-plan.md` is the authoritative specification; this file tells you how to work within it.

If you only read one section, read **Determinism** below. It is the reason this project exists.

---

## 1. What this system is

ClinRand generates randomization lists for clinical trials. A list produced here determines which treatment real patients receive, and it is consumed downstream by an IWRS that cannot detect an error in it.

Two consequences for how you work:

1. **A wrong list is undetectable.** A shuffled list and a correctly randomized list look identical. There is no test a reviewer can run that catches a subtly biased algorithm by inspection. The validation suite is the only defence, which is why it is treated as sacred here.
2. **Reproducibility is a regulatory obligation, not a nice-to-have.** Years from now someone may be asked to regenerate a list from its manifest and prove it matches. Any change that silently alters generated output destroys that ability.

When unsure about anything in §3 or §4 below, stop and ask. A question costs minutes.

---

## 2. Read order

1. This file.
2. `docs/plans/clinrand-implementation-plan.md` — the specification. Section numbers referenced below are from this document.
3. `docs/determinism.md` — the algorithm contract, once it exists.
4. `validation/README.md` — what each validation tier means and does not mean.

Do not start a phase before the previous phase's "Done when" criteria in §12 of the plan are met.

---

## 3. Determinism — the core contract

A given `(config, seed)` pair must produce a byte-identical list on every platform, on every run, forever, until `ALGO_VERSION` is deliberately incremented.

### Binding rules

- **RNG:** `rand_chacha::ChaCha20Rng` only, pinned with an exact version requirement (`=x.y.z`). `Cargo.lock` is committed.
- **Forbidden:** `rand::thread_rng`, `StdRng`, `SmallRng`, `rand::random`, `OsRng` in the allocation path, and any RNG sourced from the environment, the clock, or thread state.
- **`getrandom` is called in exactly one place:** drawing a fresh 256-bit seed at the start of a generation run.
- **Uniform draws** use the crate's own `uniform_below` with rejection sampling, exactly as specified in plan §2.2. Do not substitute `Rng::gen_range` — its internals are not stable across `rand` versions.
- **Permutation** is Fisher–Yates, descending index, exactly as specified in plan §2.3.
- **Stream consumption order** is specified in plan §2.4 and is part of the contract. Block size is drawn before the permutation. One RNG instance per run, shared across strata in canonical order.
- **Canonical stratum order** (plan §5.4) is the order given in the config, last factor varying fastest. **Do not sort it.** An agent's instinct to sort for determinism is exactly wrong here — the config order *is* the determinism.
- **No floating point** anywhere in the allocation path. No `f32`, no `f64`. Ratios, block sizes, and counts are integers.

### `ALGO_VERSION`

A plain integer in `clinrand-core`, separate from the crate version.

- Increment it when **anything** in the rules above changes, including a change that only alters how many bytes are consumed.
- Do **not** increment it for report formatting, CLI flags, UI work, or documentation.
- Never change it as a side effect of another change. It gets its own commit, with a changelog entry and a new regression fixture set.

`clinrand reproduce` refuses outright when a manifest's `algo_version` differs from the binary's. **Do not add a `--force` flag, a `--skip-version-check`, or any other bypass.** This is the most likely "helpful" thing you will be tempted to do when you hit the refusal, and it defeats the entire purpose of the field.

---

## 4. Hard prohibitions

Never do any of these without explicit written instruction from the project owner.

1. **Never regenerate a regression fixture's expected hash to make CI pass.** A broken fixture means either the change is wrong or `ALGO_VERSION` needs to increment and a new fixture set added under `validation/regression/algo-vN/`. The old fixtures stay.
2. **Never invent an expected value for `validation/reference/`.** Those come from a citable external source — RFC 8439 test vectors, hand-worked examples documented in `reference-output.md`. If you cannot cite it, the case does not belong in that tier.
3. **Never move a case between validation tiers** to make it pass. The tiers carry different evidential weight and mixing them is a misrepresentation.
4. **Never add network capability, HTTP client, telemetry, crash reporting, or an auto-updater.** The regulatory posture is that this application is provably incapable of transmitting a list.
5. **Never add Tauri mobile targets.**
6. **Never weaken the Tauri capability configuration.** No `http`, no `shell`, no `process`, no `updater`. `fs` scoped to dialog-returned paths only — no static allowlist, no `$HOME` wildcard.
7. **Never put allocation, hashing, or canonicalization logic in the CLI or the frontend.** It lives in `clinrand-core` and `clinrand-package`.
8. **Never write an allocation to the blinded report.** `generation-report.html` contains configuration, counts, structure, and hashes — never an arm assignment for any randomization number.
9. **Never write the seed to the blinded manifest**, a log line, an error message, a crash dump, or the terminal. The seed is equivalent to the list.
10. **Never commit real study data.** Configs, lists, seeds, and fixtures are synthetic, with study IDs matching `^(DEMO|TEST|EXAMPLE)-`.
11. **Never implement minimization, biased-coin, big-stick, or any runtime allocation algorithm.** Out of scope for v1.
12. **Never run `cargo update`** or relax a version pin without review. Each dependency version is validation surface.
13. **Never add `unsafe`.** Every crate carries `#![forbid(unsafe_code)]`.
14. **Never add a dependency to the emitted `qc.R`** beyond base R, `jsonlite`, and `digest`.

---

## 5. Crate boundaries

| Crate | May depend on | Must not |
|---|---|---|
| `clinrand-core` | `rand_chacha`, `serde`, `thiserror` | Touch the filesystem, read the clock, read env vars, call `getrandom`, know what a path is |
| `clinrand-package` | `clinrand-core`, `serde_json`, `sha2`, `chrono`, templating | Implement any allocation logic |
| `clinrand-cli` | both above, `clap` | Implement any allocation or hashing logic |
| `apps/desktop` | core and package via Tauri commands | Implement any allocation logic in TypeScript |

`clinrand-core::generate` is a pure function of `(config, seed)`. It takes no clock, no paths, no operator name. Enforce this mechanically: `clinrand-core` must contain no `use std::fs`, no `std::time`, no `std::env`.

If you find yourself needing I/O inside `clinrand-core` to make something work, the design is wrong — surface the problem rather than adding the import.

---

## 6. Rust conventions

- Edition 2021 or later, `#![forbid(unsafe_code)]`, `#![deny(clippy::all)]`, CI runs `clippy -- -D warnings`.
- No `.unwrap()` or `.expect()` in library code. Errors are typed with `thiserror` and propagated. `unwrap` is acceptable in tests and in `main` where a panic is the intended behaviour.
- No `as` casts between integer widths in the allocation path. Use `try_into()` and handle the error.
- Integer arithmetic in the allocation path uses checked or saturating operations explicitly. An overflow must never wrap silently.
- Public API items carry doc comments. Anything in the determinism contract carries a doc comment that states it is contract-bound and must not be changed without an `ALGO_VERSION` bump.
- `rustfmt` default configuration, enforced in CI.
- Dependencies are added only with a stated justification in the PR. Prefer the standard library. Prefer no dependency at all.

---

## 7. The three validation tiers

These are not three folders of tests. They are three different kinds of evidence, and conflating them is the worst thing you can do to this repository.

| Tier | What it proves | Where expected values come from |
|---|---|---|
| `validation/reference/` | **Correctness** against an external normative source | RFC 8439 vectors; hand-worked examples with cited derivation |
| `validation/properties/` | **Invariants hold** for arbitrary valid configurations | Nothing — asserts properties, not values |
| `validation/regression/` | **Nothing changed** since the last approved version | This engine's own prior output |

Regression fixtures prove consistency, never correctness. Never describe them otherwise in code, comments, docs, or a PR description. `validation/README.md` must state this distinction plainly; keep it accurate.

Property tests run a minimum of 1000 cases in CI. Do not reduce this number to speed up a build.

---

## 8. Testing requirements

Required and non-negotiable:

- ChaCha20 keystream matches RFC 8439 vectors exactly.
- `uniform_below` matches hand-worked rejection-sampling cases, including the zero-consumption case at `n == 1`.
- `generate(cfg, seed)` called twice returns identical output.
- A one-bit change in the seed produces a different list.
- Property checks P01–P09 (plan §7) hold across the full proptest sweep.
- Every regression fixture's `list_sha256` matches.
- The blinded report contains no randomization-number-to-arm pairing. Implement this as a real test: for every randomization number in the list, assert no line of the rendered blinded report contains both that number and any arm code.
- The emitted `qc.R` runs clean on a synthetic package and reports FAIL on a deliberately corrupted `list.csv`.
- `reproduce` refuses an `algo_version` mismatch with exit code 4.

Never mark any of these `#[ignore]`.

---

## 9. When a test fails

In order:

1. Assume the code is wrong.
2. If you are confident the test is wrong, explain why in the PR and wait for a decision. Do not edit the test yourself.
3. Do not add a tolerance, a retry, or a randomized-seed workaround to make a deterministic test pass. Non-determinism in this codebase is a bug by definition.
4. If a property test fails on a shrunk case you believe is an invalid config, fix `validate_config` to reject it — do not narrow the proptest strategy to avoid generating it.

---

## 10. Working method

- One phase per branch, one PR per phase, referencing which plan requirements it implements.
- Where the plan is ambiguous, write your reading in the PR description and flag it. Do not resolve ambiguity silently.
- Where you believe the plan is wrong, say so before implementing. Do not build something you think is unsafe because a document says to.
- Record any decision not dictated by the plan in `docs/decisions/` as a short numbered entry: the decision, the alternatives, the reason.
- Keep `CHANGELOG.md` current. Any `ALGO_VERSION` change gets a prominent entry.

---

## 11. Context you should not have to rediscover

- **This tool does not allocate subjects.** It generates a list offline, ahead of time, for a statistician to QC and approve. Runtime allocation is the job of **myIWRS**, a separate web application. If you are writing code that assigns a specific patient to an arm, you are in the wrong repository.
- **The output package is a published interface** consumed by myIWRS. Changing `list.csv` columns, manifest fields, or the canonical JSON rules breaks a downstream system. Treat the format in plan §6 as versioned and stable.
- **The repository is public.** That is fine — the algorithm is not the secret, the seed is. Never let the seed, or anything derived from it that would permit regeneration, reach a log, a fixture, an issue, or a commit.
- **The user is one unblinded statistician on one machine**, running this a handful of times per study, who will later be asked by an inspector to demonstrate that the list in the trial is the list this tool produced. Design every output with that conversation in mind.
