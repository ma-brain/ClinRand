//! Stream-consumption log for allocation-path draws (plan §4).

/// Ordered record of accepted [`super::uniform_below`] draws.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StreamLog {
    /// Accepted draws in the order they were returned.
    pub draws: Vec<StreamDraw>,
}

/// One accepted uniform draw.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamDraw {
    /// Zero-based index in [`StreamLog::draws`].
    pub index: u64,
    /// Exclusive upper bound passed to `uniform_below`.
    pub bound: u64,
    /// Returned value, `x % bound`.
    pub value: u64,
    /// Why the caller requested the draw.
    pub purpose: DrawPurpose,
}

/// Caller-supplied purpose for a recorded draw (plan §4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawPurpose {
    BlockSize,
    Permutation,
    SimpleAllocation,
}
