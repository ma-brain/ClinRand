# 0008 — `decrypt` command, in-place decryption, and passphrase UX

**Date:** 2026-09-15

## Decision

1. **New CLI subcommand: `clinrand decrypt --package <dir>`.** Not in the
   original plan §10 command list (written before Phase 8 existed).
   `--encrypt` is unusable without an inverse operation — `qc.R` and the
   unblinded report both need the plaintext restricted files at some point —
   so this addition is necessary, not optional. Flagging it here per
   AGENTS.md §10 ("record any decision not dictated by the plan").
2. **Decrypts in place**, writing the 5 restricted files directly into
   `--package` rather than to a separate `--out` directory. `qc.R` reads
   `list.csv` / `stream.csv` / `manifest.unblinded.json` "from its own
   directory" (plan §9.1); decrypting elsewhere would mean copying `qc.R`
   alongside the decrypted files or teaching it a new path convention.
   In-place decryption means every existing consumer (`qc.R`, `verify`, the
   unblinded report viewer) works unchanged once decrypted. Refuses to
   overwrite any restricted file that already exists.
3. **`generate --encrypt` prompts for the passphrase twice** (entry +
   confirmation); **`decrypt` prompts once.** A typo'd passphrase on encrypt
   is unrecoverable — there is no way to regenerate `restricted.age` short
   of re-running `generate` (a new seed, a new list). Decrypt failure is
   cheap to retry, so a single prompt is enough.
4. **New exit code 5 (`PassphraseFailure`)**, covering a mismatched
   confirmation on `generate --encrypt` and a wrong passphrase / corrupt
   container on `decrypt`. Distinct from exit 1 (check failure) so scripts
   can tell "the passphrase step failed" from "the list or checksums are
   wrong." Documented in `docs/cli.md` alongside the existing 0–4.
5. **Non-interactive stdin fallback.** `rpassword` reads from `/dev/tty` (or
   the platform equivalent) when stdin is a terminal. When stdin is piped or
   redirected, there is no terminal to hide input on regardless — this falls
   back to a plain line read from stdin, the same pattern `age` and
   `ssh-keygen` use for scripted / non-interactive invocation. This is also
   what makes `--encrypt` / `decrypt` testable at all: the integration tests
   spawn `clinrand` with piped stdin and feed the passphrase exactly as a
   script would.

## Alternatives

- **`--passphrase` flag or an env var.** Rejected: both leak into shell
  history, process listings (`ps`), or CI logs — exactly the class of
  exposure the seed-secrecy rule (AGENTS.md §4.9) exists to prevent, applied
  here to the passphrase.
- **Decrypt to a required `--out` directory**, mirroring `generate`/
  `reproduce`. Rejected per point 2 above — it would make the natural
  "decrypt, then run qc.R" workflow two-step (decrypt, then copy `qc.R` in).
- **Single passphrase prompt on encrypt, like `decrypt`.** Rejected:
  `generate` draws a fresh seed and produces a new list either way, but a
  passphrase typo specifically loses the encryption key with no recovery
  path — worth the extra prompt.

## Reason

The desktop app makes the same decrypt-in-place choice for the same reason
(`apps/desktop/src-tauri/src/commands/decrypt.rs`): one code path, one
mental model, whether the operator is at the CLI or in the GUI.
