# 0003 — validate_config error collection and overflow

**Date:** 2026-09-15

## Decision

`validate_config` returns `Result<Vec<ConfigWarning>, Vec<ConfigError>>`
and collects every independent §5.2 failure rather than stopping at the
first. `ValidateOptions { allow_large_strata }` is the only override for
the 200-combination sanity guard.

A factor with an empty `levels` array is a `ConfigError` (no Cartesian
combinations can be formed). Integer overflow of the ratio sum or of the
combination count is a `ConfigError` and is never wrapping; combination
overflow cannot be overridden with `allow_large_strata`.

`simple` with non-empty `strata` is accepted. Variable `sizes` may not
exceed 24; that cap is not applied to a fixed block size.

## Alternatives

- Fail-fast on the first error.
- Treat empty `levels` as zero combinations and accept the config.
- Wrap combination counts at `u32::MAX` and then apply the 200 guard.

## Reason

Inspectors and authors need every problem in one pass. Empty levels and
overflowed counts are not representable stratum spaces. The 24 cap is
worded in the plan against `sizes`, not `size`.
