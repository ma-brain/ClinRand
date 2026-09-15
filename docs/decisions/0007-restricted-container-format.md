# 0007 — `restricted.age` container format

**Date:** 2026-09-15

## Decision

`--encrypt` (plan §6.6) wraps the five restricted files (`list.csv`,
`list.json`, `manifest.unblinded.json`, `stream.csv`,
`unblinded-report.html`) into a single file, `restricted.age`, using a
custom binary container:

```
magic       4 bytes   b"CRV1"
kdf_m_cost  u32 LE     Argon2id memory cost (KiB)
kdf_t_cost  u32 LE     Argon2id iterations
kdf_p_cost  u32 LE     Argon2id parallelism
salt        16 bytes   random, unique per encryption
nonce       24 bytes   random, unique per encryption (XChaCha20 nonce)
ciphertext  remainder  AEAD output (includes the 16-byte Poly1305 tag)
```

The plaintext wrapped by the AEAD is a minimal multi-file archive (sorted by
name, no external dependency): `u16 name_len | name (UTF-8) | u64
content_len | content`, repeated per file. AEAD associated data is
`"clinrand-restricted-v1:" + study_id + ":" + list_sha256`, binding the
container to its specific package.

Crates: `chacha20poly1305` (XChaCha20-Poly1305) and `argon2` (Argon2id),
exact-pinned in `crates/clinrand-package/Cargo.toml`, matching the plan's
named algorithms. `zeroize` wipes the derived key and passphrase copies.

Argon2id defaults: `m_cost=131072` (128 MiB), `t_cost=3`, `p_cost=4`,
32-byte key — stored in the header, not hard-coded, so future containers can
retune cost without a new magic. This format is versioned independently of
`ALGO_VERSION`, which governs allocation determinism only (AGENTS.md §3);
encryption is a package/file-format concern.

## Alternatives

- **Literal `age` format / the `age` crate.** Plan §6.6 says
  "`restricted.age`-style container," not literal `age`-CLI compatibility,
  and explicitly names XChaCha20-Poly1305 + Argon2id — `age`'s passphrase
  mode uses scrypt, not Argon2id, and ChaCha20-Poly1305 (12-byte nonce), not
  XChaCha20. Matching the plan's named primitives meant a custom container,
  not the real `age` format.
- **One `.age`-style file per restricted file** instead of one archive.
  Rejected: five separate containers means five passphrase prompts' worth of
  KDF cost, and `checksums.txt` would need five new lines instead of one.
  Bundling matches "wraps the restricted files ... into a container"
  (singular) in the plan text.
- **A general archive format (tar, zip)** for the pre-encryption bundle.
  Rejected per AGENTS.md §6 ("prefer no dependency at all") — the format is
  five files with known names; a 12-byte-overhead-per-entry TLV format needs
  no library and is trivial to describe precisely in `docs/output-package.md`.

## Reason

The plan names the algorithms explicitly, which is a stronger signal than
the word "age" — treat "age-style" as "an age-like sealed container," not a
compatibility requirement. Storing KDF params in the header (rather than a
fixed constant known only to the binary) means a future cost retune doesn't
strand old containers, mirroring how `ALGO_VERSION` already handles this
class of problem for the allocation contract. Binding the AAD to
`study_id` + `list_sha256` costs nothing and closes an otherwise-possible
container-substitution mistake (copying a `restricted.age` from one package
directory into another and having it silently decrypt).
