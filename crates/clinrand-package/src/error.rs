//! Errors from package rendering and (later) writing.

use std::fmt;

/// Failure producing package file bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PackageError {
    /// An allocation record is missing a stratum factor required by the config.
    MissingStratumFactor {
        /// Factor name from `StudyConfig.strata` (config order).
        factor: String,
        /// Randomization number of the incomplete record.
        randomization_number: String,
    },
}

impl fmt::Display for PackageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingStratumFactor {
                factor,
                randomization_number,
            } => write!(
                f,
                "allocation {randomization_number} is missing stratum factor `{factor}`"
            ),
        }
    }
}

impl std::error::Error for PackageError {}
