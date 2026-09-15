# 0004 — Reject block size 0

**Date:** 2026-09-15

## Decision

`validate_config` rejects `fixed { size: 0 }` and any `0` in variable
`sizes` as `ConfigError::BlockSizeZero`. The published schema sets
`"minimum": 1` on block `size` and `sizes[]`.

## Alternatives

- Leave `0` to the existing multiple-of-ratio-sum check.
- Treat `0` as `EmptyBlockSizes` / `BlockSizeTooLarge`.

## Reason

Plan §5.2 does not name size `0`, but `0 % n == 0` for every ratio sum,
so the multiple check accepts a zero-length block. That is not a
meaningful block and would consume the allocation stream incorrectly.
A dedicated error is clearer than overloading the multiple-of-sum
variants.
