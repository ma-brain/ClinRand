//! Errors from package rendering and writing.

use std::fmt;
use std::io;

use crate::canonical::CanonicalError;

/// Failure producing or writing package file bytes.
///
/// [`Display`](fmt::Display) never includes the seed.
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
    /// Filesystem create/write failed while assembling a package directory.
    Io(io::Error),
}

impl From<CanonicalError> for PackageError {
    fn from(err: CanonicalError) -> Self {
        Self::Canonical(err)
    }
}

impl From<io::Error> for PackageError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
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
            Self::Io(err) => write!(f, "package I/O failed: {err}"),
        }
    }
}

impl std::error::Error for PackageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::MissingStratumFactor { .. } => None,
            Self::Canonical(err) => Some(err),
            Self::Io(err) => Some(err),
        }
    }
}
