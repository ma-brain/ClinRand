# Uniform-below reference output

Expected integers in `cases.json` are derived on paper from a
**documented keystream** of unsigned 64-bit words. They were not
produced by this engine.

`uniform_below` draws `x = next_u64()`, rejects `x` when `x >= zone`,
and otherwise returns `x % n`, with:

```text
zone = u64::MAX - (u64::MAX % n)
```

(`n == 0` is an error; `n == 1` returns 0 and consumes no words.)
`u64::MAX` is 18446744073709551615.

StreamLog records **accepted** draws only. Discarded samples still
consume keystream words (`consumed` counts every `next_u64`).

## Documented keystream

These words are the input to the cases, chosen so the arithmetic is
visible by hand. They are not ChaCha20 output.

| Word | Decimal | Role |
|---|---|---|
| `k0` | 5 | accepted by `n = 3`; left unused by `n = 0` and `n = 1` |
| `k1` | 18446744073709551614 (`u64::MAX - 1`) | rejected by `n = 2` |
| `k2` | 7 | accepted by `n = 2` after the rejection |

Each case names its own `input.keystream` prefix of this sequence.

## `n_eq_0_error` — plan §2.2 step 1

- `n = 0`
- keystream: `[5]`
- Result: error `zero_bound`
- Words consumed: 0 (the `5` remains)
- StreamLog: empty

## `n_eq_1_zero_consumption` — plan §2.2 step 1

- `n = 1`
- keystream: `[5]`
- Result: `0`
- Words consumed: 0 (the `5` remains)
- StreamLog: empty (`n == 1` does not record a draw)

## `n_eq_3_no_rejection` — plan §2.2 steps 2–3

- `n = 3`
- keystream: `[5]`

`u64::MAX % 3`: `2 ≡ −1 (mod 3)`, so `2^64 ≡ (−1)^64 ≡ 1 (mod 3)` and
`2^64 − 1 ≡ 0 (mod 3)`. Therefore `MAX % 3 = 0` and
`zone = MAX − 0 = MAX`.

- `x = 5`. `5 < zone`, accept.
- `5 % 3 = 2`.

Words consumed: 1. StreamLog: one accepted draw
`{index: 0, bound: 3, value: 2, purpose: Permutation}`.

## `n_eq_2_rejection` — plan §2.2 step 3

- `n = 2`
- keystream: `[18446744073709551614, 7]`

`u64::MAX` is odd, so `MAX % 2 = 1` and
`zone = MAX − 1 = 18446744073709551614`.

- `x0 = 18446744073709551614`. `x0 >= zone`, discard.
- `x1 = 7`. `7 < zone`, accept.
- `7 % 2 = 1`.

Words consumed: 2. StreamLog: one accepted draw
`{index: 0, bound: 2, value: 1, purpose: BlockSize}`.

Comparison is exact integer equality; there is no tolerance.
