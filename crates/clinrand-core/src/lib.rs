//! Deterministic randomization-list engine.
//!
//! This crate must remain free of filesystem, clock, and environment
//! access, and must not draw OS entropy. `generate` is a pure function of
//! `(config, seed)`.

#![forbid(unsafe_code)]
#![deny(clippy::all)]

/// Algorithm identity for the allocation path.
///
/// Contract-bound: increment this only when the determinism contract
/// changes (RNG, uniform draws, permutation, stream order, or the
/// no-float rule), including changes that only alter how many bytes
/// are consumed. Must not change as a side effect of another change.
/// Bumps require their own commit, a changelog entry, and a new
/// regression fixture set. See plan §2.6.
pub const ALGO_VERSION: u32 = 1;

mod config;
mod rng;
mod stream;
mod uniform;
mod validate;

pub use config::{Arm, BlockScheme, Method, NumberingScheme, StratificationFactor, StudyConfig};
pub use rng::Rng;
pub use stream::{DrawPurpose, StreamDraw, StreamLog};
pub use uniform::{uniform_below, U64Draw, UniformError};
pub use validate::{validate_config, ConfigError, ConfigWarning, ValidateOptions};

#[cfg(test)]
mod tests {
    use super::ALGO_VERSION;

    #[test]
    fn algo_version_starts_at_one() {
        assert_eq!(ALGO_VERSION, 1);
    }
}
