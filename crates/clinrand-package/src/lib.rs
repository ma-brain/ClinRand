//! Output package reading, writing, hashing, and reports.
//!
//! Allocation logic lives in `clinrand-core` and must not be implemented here.

#![forbid(unsafe_code)]
#![deny(clippy::all)]

pub use clinrand_core::ALGO_VERSION;

#[cfg(test)]
mod tests {
    #[test]
    fn reexports_core_algo_version() {
        assert_eq!(clinrand_core::ALGO_VERSION, 1);
    }
}
