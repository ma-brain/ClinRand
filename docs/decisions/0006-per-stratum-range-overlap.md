# 0006 — Per-stratum range overlap is not a validate reject

**Date:** 2026-09-15

## Decision

Phase 3 does **not** invent a `validate_config` reject when
`per_stratum_range` reserved ranges would overlap (plan §5.6). Overlap is a
known gap: `block_size` smaller than `list_length_per_stratum` can assign the
same randomization numbers to more than one stratum. Generation still runs;
property check **P04** (no duplicate randomization numbers) fails on the
resulting list.

## Alternatives

- Reject overlapping ranges in `validate_config` (require
  `block_size >= list_length_per_stratum`, or compute non-overlap from
  stratum count).
- Silently widen ranges or renumber to avoid collisions.

## Reason

Plan §5.6 documents disclosure risk and the opt-in warning posture; it does
not specify an overlap reject. Inventing one in Phase 3 would be a silent
spec extension. Leaving detection to P04 keeps the gap visible without
pretending validation closed it.
