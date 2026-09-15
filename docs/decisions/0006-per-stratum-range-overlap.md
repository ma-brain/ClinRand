# 0006 — Reject overlapping `per_stratum_range` reserved ranges

**Date:** 2026-09-15  
**Updated:** 2026-09-15 (owner decision: add §5.2 reject)

## Decision

`validate_config` **rejects** `per_stratum_range` when
`block_size < list_length_per_stratum`. Adjacent strata would share
randomization numbers (`start + s * block_size` ranges overlap), which
breaks uniqueness that IWRS and kit logistics assume.

Disclosure of stratum membership via the number remains a **warning** only
(plan §5.6). Overlap is a different failure mode and is an error.

## Alternatives

- Detect only via property check P04 after generation.
- Silently widen `block_size` or renumber to avoid collisions.
- Leave as a known gap with no validate reject (earlier Phase 3 reading).

## Reason

Duplicate randomization numbers are not a soft QC issue. Refusing at
config time is cheaper and clearer than emitting a list that fails P04.
The rule is a single integer comparison and matches how reserved ranges
are meant to work: each stratum owns a contiguous block large enough for
its list. Owner directed this into plan §5.2 and the Phase 3 branch.
