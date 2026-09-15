# ClinRand operator handbook

This handbook is for the **unblinded statistician** who plans, generates,
QCs, and approves a randomization list with ClinRand — on one machine, a
handful of times per study.

- **[`operator-workflow.md`](operator-workflow.md)** — one continuous
  walkthrough of a complete study, from writing a config through to a
  formally approved, archived package.

The handbook deliberately does not duplicate documents that already cover
their piece precisely:

| Document | Covers |
|---|---|
| [`docs/qc-procedure.md`](../docs/qc-procedure.md) | Running `qc.R`, interpreting PASS/FAIL, what to do on a failure, seed handling during QC |
| [`docs/cli.md`](../docs/cli.md) | Every CLI command, flag, and exit code |
| [`docs/output-package.md`](../docs/output-package.md) | The package file formats, hashes, and the `restricted.age` container, precisely enough to reimplement |
| [`docs/security-posture.md`](../docs/security-posture.md) | The no-network-capability claim and how to verify it |
| [`apps/desktop/README.md`](../apps/desktop/README.md) | Desktop app build, dev, and manual smoke-test flow |
| [`validation/README.md`](../validation/README.md) | The three validation tiers and their differing evidential weight |

The scope statement in the project [`README.md`](../README.md) applies
throughout this handbook: **ClinRand produces no approved artifact on its
own.** A list it generates must be independently QC'd and formally approved
by a qualified unblinded statistician before any use in a clinical trial.
The evidence this tool produces exists to make that review possible;
responsibility for the list rests with the statistician who approves it,
not with the software.
