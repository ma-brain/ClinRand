//! Errors from package rendering and (later) writing.

use std::fmt;

use crate::canonical::CanonicalError;

/// Failure producing package file bytes.
#[derive(Debug)]
pub enum PackageError {
    /// An allocation record is missing a stratum factor required by the config.
    MissingStratumFactor {
        /// Factor name from `StudyConfig.strata` (config order).
        factor: String,
        /// Randomization number of the incomplete record.
        randomization_number: String,
    },
    /// Canonical JSON or `config_sha256` failed.
    Canonical(CanonicalError),
}

impl From<CanonicalError> for PackageError {
    fn from(err: CanonicalError) -> Self {
        Self::Canonical(err)
    }
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
            Self::Canonical(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for PackageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::MissingStratumFactor { .. } => None,
            Self::Canonical(err) => Some(err),
        }
    }
}
