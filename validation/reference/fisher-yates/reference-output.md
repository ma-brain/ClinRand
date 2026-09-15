# Fisher–Yates reference output

Expected permutations in `cases.json` are derived on paper from a
**documented keystream** of unsigned 64-bit words, using the
rejection-sampling rules of `uniform_below` (see
[`../uniform-below/reference-output.md`](../uniform-below/reference-output.md)).
They were not produced by this engine.

Descending Fisher–Yates (plan §2.3 / `docs/determinism.md` §4):

```text
for i in (1..len).rev():
    j = uniform_below(rng, log, i + 1, Permutation)
    swap(items[i], items[j])
```

`len == 0` or `len == 1` performs no draws and leaves the slice unchanged.
Every accepted draw records `purpose: Permutation`. StreamLog records
**accepted** draws only; `consumed` counts every `next_u64`.

`u64::MAX` is 18446744073709551615.

## Documented keystream

These words are the input to the cases, chosen so the arithmetic is
visible by hand. They are not ChaCha20 output.

| Word | Decimal | Role |
|---|---|---|
| `k0` | 5 | unused by length-0 / length-1 cases |
| `k1` | 6 | accepted by `n = 2` (`6 % 2 = 0`) and by `n = 2` in the length-3 case |
| `k2` | 4 | accepted by `n = 3` (`4 % 3 = 1`) |

Each case names its own `input.keystream` prefix of this vocabulary.

For every bound used below (`n = 2` or `n = 3`), the first keystream word
is strictly less than `zone`, so there is no rejection step.

- `n = 3`: `MAX % 3 = 0`, so `zone = MAX`. Any `x < MAX` is accepted.
- `n = 2`: `MAX % 2 = 1`, so `zone = MAX - 1 = 18446744073709551614`.
  Words `4` and `6` are both `< zone`.

## `len_0_identity` — plan §2.3 empty slice

- items: `[]`
- keystream: `[5]`
- Loop `(1..0).rev()` is empty.
- Result: `[]`
- Words consumed: 0. StreamLog: empty.

## `len_1_zero_consumption` — plan §2.3 length 1

- items: `[42]`
- keystream: `[5]`
- Loop `(1..1).rev()` is empty.
- Result: `[42]`
- Words consumed: 0. StreamLog: empty.

## `len_2_one_draw_swap` — plan §2.3 length 2 with `j != i`

- items: `[0, 1]`
- keystream: `[6]`

Only `i = 1`:

1. `j = uniform_below(..., n = 2)`. `x = 6`, `6 < zone`, `6 % 2 = 0`.
2. `swap(items[1], items[0])` → `[1, 0]`.

Words consumed: 1. StreamLog: one accepted draw
`{index: 0, bound: 2, value: 0, purpose: Permutation}`.

## `len_3_descending_swaps` — plan §2.3 length 3 with `j != i`

- items: `[0, 1, 2]`
- keystream: `[4, 6]`

1. `i = 2`: `j = uniform_below(..., n = 3)`. `x = 4`, `4 % 3 = 1`.
   `swap(items[2], items[1])` → `[0, 2, 1]`.
2. `i = 1`: `j = uniform_below(..., n = 2)`. `x = 6`, `6 % 2 = 0`.
   `swap(items[1], items[0])` → `[2, 0, 1]`.

Words consumed: 2. StreamLog:

- `{index: 0, bound: 3, value: 1, purpose: Permutation}`
- `{index: 1, bound: 2, value: 0, purpose: Permutation}`

Comparison is exact integer equality of the final order and of each
accepted stream entry; there is no tolerance.
