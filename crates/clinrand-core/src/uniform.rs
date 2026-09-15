//! Rejection-sampling uniform integer draw (plan §2.2).

use crate::rng::Rng;
use crate::stream::{DrawPurpose, StreamDraw, StreamLog};

/// Source of 64-bit words for [`uniform_below`].
///
/// Contract-bound: the allocation path uses [`Rng`]. A documented
/// integer keystream may implement this trait for reference cases.
pub trait U64Draw {
    /// Draw the next 64 bits.
    fn next_u64(&mut self) -> u64;
}

impl U64Draw for Rng {
    fn next_u64(&mut self) -> u64 {
        Rng::next_u64(self)
    }
}

/// Why [`uniform_below`] refused to draw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UniformError {
    /// `n == 0` is not a valid exclusive upper bound.
    ZeroBound,
    /// The stream log has more draws than `u64` can index.
    StreamIndexOverflow,
}

impl std::fmt::Display for UniformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroBound => write!(f, "uniform_below bound must be greater than zero"),
            Self::StreamIndexOverflow => write!(f, "stream log index overflowed u64"),
        }
    }
}

impl std::error::Error for UniformError {}

/// Draw an integer in `0..n` by rejection sampling.
///
/// Contract-bound: must not change without an `ALGO_VERSION` bump.
/// Production draws use [`Rng::from_seed`] plus [`Rng::next_u64`].
/// `n == 1` consumes no bytes and returns 0. `n == 0` is an error.
pub fn uniform_below<R: U64Draw>(
    rng: &mut R,
    log: &mut StreamLog,
    n: u64,
    purpose: DrawPurpose,
) -> Result<u64, UniformError> {
    if n == 0 {
        return Err(UniformError::ZeroBound);
    }
    if n == 1 {
        return Ok(0);
    }

    let remainder = u64::MAX % n;
    let zone = u64::MAX.saturating_sub(remainder);
    loop {
        let x = rng.next_u64();
        if x >= zone {
            continue;
        }
        let value = x % n;
        record_draw(log, n, value, purpose)?;
        return Ok(value);
    }
}

fn record_draw(
    log: &mut StreamLog,
    bound: u64,
    value: u64,
    purpose: DrawPurpose,
) -> Result<(), UniformError> {
    let index = u64::try_from(log.draws.len()).map_err(|_| UniformError::StreamIndexOverflow)?;
    log.draws.push(StreamDraw {
        index,
        bound,
        value,
        purpose,
    });
    Ok(())
}
