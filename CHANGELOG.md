# Changelog

All notable changes to ClinRand are documented here.

`ALGO_VERSION` is independent of the crate version. A bump of that
integer gets a prominent entry and a new regression fixture set; it
is never changed as a side effect of another change.

## Unreleased

### Added

- Phase 0 workspace scaffold: `clinrand-core`, `clinrand-package`,
  `clinrand-cli`, `justfile`, and CI on Linux, macOS, and Windows.
- ChaCha20 generator in `clinrand-core` (`rand_chacha = "=0.3.1"`)
  and RFC 8439 §2.3.2 / §2.4.2 reference cases under
  `validation/reference/chacha20/`.
