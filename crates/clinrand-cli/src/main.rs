//! Command-line interface for ClinRand.
//!
//! Allocation and hashing logic live in `clinrand-core` and
//! `clinrand-package`. This binary must not reimplement them.

#![forbid(unsafe_code)]
#![deny(clippy::all)]

fn main() {
    let algo = clinrand_core::ALGO_VERSION;
    assert_eq!(algo, clinrand_package::ALGO_VERSION);
    println!(
        "clinrand {version} (algo_version {algo})",
        version = env!("CARGO_PKG_VERSION"),
    );
}
