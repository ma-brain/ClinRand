//! OS entropy seed draw for list generation.
//!
//! This is the only `getrandom` call site in the workspace source.

/// Draw a fresh 256-bit seed from the operating system.
///
/// # Errors
///
/// Returns [`getrandom::Error`] when the platform RNG is unavailable.
pub fn draw_seed() -> Result<[u8; 32], getrandom::Error> {
    let mut seed = [0u8; 32];
    getrandom::fill(&mut seed)?;
    Ok(seed)
}

#[cfg(test)]
mod tests {
    #[test]
    fn draw_seed_returns_32_bytes() {
        let seed = super::draw_seed().expect("draw_seed");
        assert_eq!(seed.len(), 32);
    }
}
