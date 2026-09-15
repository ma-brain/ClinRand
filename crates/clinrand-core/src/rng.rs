//! ChaCha20 generator used by the allocation path.
//!
//! Production construction is [`Rng::from_seed`]. IETF nonce/counter
//! positioning exists only so RFC 8439 reference cases can be checked
//! against the same `rand_chacha::ChaCha20Rng` type.

use rand_chacha::ChaCha20Rng;
use rand_core::{RngCore, SeedableRng};

/// Deterministic ChaCha20 generator used by the allocation path.
///
/// Contract-bound: production construction is [`Rng::from_seed`] with
/// stream 0. Changing the RNG type, the meaning of the 256-bit seed, or
/// how bytes are drawn requires an `ALGO_VERSION` bump.
pub struct Rng {
    inner: ChaCha20Rng,
}

impl Rng {
    /// Seed the generator from a 256-bit key. Stream and word position
    /// start at 0.
    ///
    /// Contract-bound: must not change without an `ALGO_VERSION` bump.
    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self {
            inner: ChaCha20Rng::from_seed(seed),
        }
    }

    /// Position the same `ChaCha20Rng` type at an IETF ChaCha20 block
    /// (RFC 8439 96-bit nonce + 32-bit block counter).
    ///
    /// `rand_chacha` 0.3.1 uses Bernstein's 64-bit counter and 64-bit
    /// stream id. The last 64 bits of the nonce become the stream; the
    /// first 32 bits of the nonce are the high half of the 64-bit
    /// counter, combined with `block_counter` and applied via
    /// `set_word_pos`. Not used by the allocation path.
    pub fn from_ietf(key: [u8; 32], nonce: [u8; 12], block_counter: u32) -> Self {
        let mut rng = Self::from_seed(key);
        rng.position_ietf(nonce, block_counter);
        rng
    }

    /// Fill `dest` with the next keystream bytes.
    ///
    /// Contract-bound: must not change without an `ALGO_VERSION` bump.
    pub fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.inner.fill_bytes(dest);
    }

    /// Draw the next 64 bits from the ChaCha20 stream.
    ///
    /// Contract-bound: production `uniform_below` consumes randomness
    /// only through this method. Must not change without an
    /// `ALGO_VERSION` bump.
    pub fn next_u64(&mut self) -> u64 {
        self.inner.next_u64()
    }

    fn position_ietf(&mut self, nonce: [u8; 12], block_counter: u32) {
        let mut stream_bytes = [0u8; 8];
        stream_bytes.copy_from_slice(&nonce[4..12]);
        self.inner.set_stream(u64::from_le_bytes(stream_bytes));

        let mut nonce_hi = [0u8; 4];
        nonce_hi.copy_from_slice(&nonce[0..4]);
        let counter_hi = u32::from_le_bytes(nonce_hi);
        let block = u64::from(block_counter) | (u64::from(counter_hi) << 32);
        let word_pos = u128::from(block).saturating_mul(16);
        self.inner.set_word_pos(word_pos);
    }
}
