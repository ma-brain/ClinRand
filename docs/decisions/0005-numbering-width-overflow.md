# 0005 — Numbering width overflow emits full decimal

**Date:** 2026-09-15

## Decision

When formatting a randomization number to `width`, if the decimal
representation of the value is longer than `width`, emit the full
decimal string without truncating digits and without panicking.
Zero-padding applies only when the decimal fits in `width`.

## Alternatives

- Panic or return `GenerationError` when the value overflows `width`.
- Truncate high-order digits to force a fixed field width.

## Reason

Plan §5.6 requires zero-padding to `width` but does not define overflow
behaviour. Truncating digits would collide distinct numbers; refusing
generation turns a presentation constraint into a hard failure for
otherwise valid configs. Emitting the unpadded full decimal preserves
identity and keeps `generate` total over validated configs.
