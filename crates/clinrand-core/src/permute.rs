//! Fisher–Yates permutation (plan §2.3).

use crate::stream::{DrawPurpose, StreamLog};
use crate::uniform::{uniform_below, U64Draw, UniformError};

/// Shuffle `items` in place with descending Fisher–Yates.
///
/// Contract-bound: must not change without an `ALGO_VERSION` bump.
/// For each `i` in `(1..len).rev()`, draws
/// `j = uniform_below(rng, log, i + 1, [`DrawPurpose::Permutation`])` and
/// swaps `items[i]` with `items[j]`. Length 0 and length 1 consume no
/// draws and leave the slice unchanged.
///
/// Production callers pass [`crate::Rng`]; reference cases may pass a
/// documented integer keystream via [`U64Draw`].
pub fn permute<R, T>(rng: &mut R, log: &mut StreamLog, items: &mut [T]) -> Result<(), UniformError>
where
    R: U64Draw,
{
    let len = items.len();
    if len <= 1 {
        return Ok(());
    }

    for i in (1..len).rev() {
        let bound_usize = i.checked_add(1).ok_or(UniformError::StreamIndexOverflow)?;
        let bound = u64::try_from(bound_usize).map_err(|_| UniformError::StreamIndexOverflow)?;
        let j_u64 = uniform_below(rng, log, bound, DrawPurpose::Permutation)?;
        let j = usize::try_from(j_u64).map_err(|_| UniformError::StreamIndexOverflow)?;
        items.swap(i, j);
    }

    Ok(())
}
