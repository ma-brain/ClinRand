//! Process exit codes (plan §10).

/// Exit codes for the `clinrand` binary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(i32)]
pub enum ExitCode {
    /// Command completed successfully.
    Success = 0,
    /// Check or verification failure.
    CheckFailure = 1,
    /// Configuration validation failure.
    #[expect(dead_code, reason = "used by validate-config in Task 2")]
    InvalidConfig = 2,
    /// I/O or parse failure.
    IoError = 3,
    /// Manifest `algo_version` differs from the binary.
    #[expect(dead_code, reason = "used by reproduce in Task 5")]
    AlgoVersionMismatch = 4,
}

impl ExitCode {
    /// Numeric exit code for [`std::process::exit`].
    pub fn code(self) -> i32 {
        self as i32
    }
}
