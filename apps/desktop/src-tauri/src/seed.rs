//! OS entropy seed draw for the desktop host.
//!
//! This is the **desktop seed-draw site**: the only `getrandom` call site in
//! the `clinrand-desktop` crate. It mirrors `clinrand-cli`'s `seed.rs`. Per
//! AGENTS.md §3, `getrandom` is called in exactly one place — drawing a fresh
//! 256-bit seed at the start of a generation run. This function is invoked
//! only from the `generate_package` command.
//!
//! The `src-tauri` crate is deliberately outside the engine workspace, so the
//! CLI and the desktop host each own their single seed-draw site (see the
//! Phase 7 ruling in `.superpowers/sdd/2026-09-15-phase-7-desktop/progress.md`).
//!
//! The drawn seed is equivalent to the list. It must never be returned to the
//! frontend, logged, or placed in an error string (AGENTS.md §4.9).

/// Draw a fresh 256-bit seed from the operating system.
///
/// # Errors
///
/// Returns [`getrandom::Error`] when the platform RNG is unavailable. The
/// error carries no seed bytes.
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
