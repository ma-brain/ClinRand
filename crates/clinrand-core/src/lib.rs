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

/// Crate version of `clinrand-core` (`CARGO_PKG_VERSION`).
///
/// Written to package manifests as `engine_version`. Distinct from
/// [`ALGO_VERSION`].
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Version string of the pinned `rand_chacha` dependency.
///
/// **Keep in sync** with the exact pin in this crate's `Cargo.toml`
/// (`rand_chacha = "=…"`; currently `=0.3.1`). Package manifests record
/// this as `rng.crate_version`.
pub const RNG_CRATE_VERSION: &str = "0.3.1";

mod config;
mod generate;
mod permute;
mod properties;
mod rng;
mod strata;
mod stream;
mod uniform;
mod validate;

pub use config::{Arm, BlockScheme, Method, NumberingScheme, StratificationFactor, StudyConfig};
pub use generate::{
    generate, generate_with_options, AllocationRecord, GeneratedList, GenerationError,
};
pub use permute::permute;
pub use properties::{check_properties, PropertyCheck, PropertyReport};
pub use rng::Rng;
pub use strata::{stratum_combinations, StratumError};
pub use stream::{DrawPurpose, StreamDraw, StreamLog};
pub use uniform::{uniform_below, U64Draw, UniformError};
pub use validate::{validate_config, ConfigError, ConfigWarning, ValidateOptions};

#[cfg(test)]
mod tests {
    use super::{ALGO_VERSION, ENGINE_VERSION, RNG_CRATE_VERSION};

    #[test]
    fn algo_version_starts_at_one() {
        assert_eq!(ALGO_VERSION, 1);
    }

    #[test]
    fn engine_version_matches_cargo_pkg_version() {
        assert_eq!(ENGINE_VERSION, env!("CARGO_PKG_VERSION"));
        assert!(!ENGINE_VERSION.is_empty());
    }

    #[test]
    fn rng_crate_version_matches_documented_pin() {
        // Must match `rand_chacha = "=0.3.1"` in Cargo.toml (see const docs).
        assert_eq!(RNG_CRATE_VERSION, "0.3.1");
    }
}
