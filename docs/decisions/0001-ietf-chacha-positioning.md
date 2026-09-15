# 0001 — IETF positioning for RFC 8439 cases

**Date:** 2026-09-15

## Decision

`Rng::from_seed` is the allocation-path constructor (stream 0, word
position 0). RFC 8439 reference tests use `Rng::from_ietf`, which
configures the same `rand_chacha::ChaCha20Rng` instance: `set_stream`
gets the last 64 bits of the 96-bit nonce; the first 32 bits of the
nonce plus the IETF block counter become the Bernstein 64-bit counter
via `set_word_pos`.

## Alternatives

- A second ChaCha implementation (IETF 32/96) used only in tests.
- `#[cfg(test)]` positioning, so integration tests could not call it.

## Reason

The controller ruling forbids a second ChaCha. `rand_chacha` 0.3.1
documents this 96-bit nonce recovery. A public constructor lets
`tests/rfc8439.rs` load `validation/reference/chacha20/cases.json`
without putting filesystem access in `src/`.
