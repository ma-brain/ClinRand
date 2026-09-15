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
    /// `PackageMeta.generated_at` is not `YYYY-MM-DDTHH:MM:SSZ`.
    InvalidGeneratedAt {
        /// The rejected timestamp string (never a seed).
        value: String,
    },
    /// Target package directory already exists; refuse overwrite.
    PackageDirExists {
        /// Absolute or relative path that already exists.
        path: std::path::PathBuf,
    },
    /// Canonical JSON or `config_sha256` failed.
    Canonical(CanonicalError),
    /// Filesystem create/write failed while assembling a package directory.
    Io(io::Error),
    /// `list.csv` is not well-formed CSV.
    ListCsvParse {
        /// Parse detail (never a seed).
        detail: String,
    },
    /// `list.csv` header does not match the expected config column order.
    ListCsvHeaderMismatch,
    /// `list.csv` has a header but no data rows.
    ListCsvNoDataRows,
    /// A data row has the wrong number of columns.
    ListCsvColumnCount {
        /// 1-based row number in the file.
        row: usize,
        /// Expected column count from the header.
        expected: usize,
        /// Actual column count.
        found: usize,
    },
    /// A numeric column in `list.csv` is not a valid `u32`.
    ListCsvInvalidInteger {
        /// Column name.
        column: String,
        /// 1-based row number in the file.
        row: usize,
    },
    /// A `checksums.txt` line is not `hex  filename` (GNU text mode).
    ChecksumsParse {
        /// 1-based line number.
        line: usize,
    },
    /// A digest on a `checksums.txt` line is not 64 lowercase hex digits.
    ChecksumsInvalidDigest {
        /// 1-based line number.
        line: usize,
    },
    /// The same path appears twice in `checksums.txt`.
    ChecksumsDuplicatePath {
        /// Duplicate relative filename.
        path: String,
    },
    /// `manifest.unblinded.json` is not valid JSON for the expected shape.
    ManifestJsonParse,
    /// `manifest.blinded.json` is not valid JSON for the expected shape.
    BlindedManifestJsonParse,
    /// `seed_hex` is not exactly 64 characters.
    InvalidSeedHexLength,
    /// `seed_hex` is not lowercase hexadecimal.
    InvalidSeedHexEncoding,
    /// A passphrase for `--encrypt` / `decrypt` was empty or whitespace-only.
    EmptyPassphrase,
    /// Argon2id key derivation failed (invalid parameters or a resource
    /// failure, e.g. the memory cost could not be allocated).
    KeyDerivationFailed,
    /// `restricted.age` failed AEAD authentication: wrong passphrase, or the
    /// container does not belong to this package. Never reveals which.
    DecryptionFailed,
    /// `restricted.age` is structurally invalid: bad magic, truncated
    /// header, a `checksums.txt` mismatch, or a malformed archive after a
    /// successful decryption. Distinct from [`Self::DecryptionFailed`] so an
    /// operator can tell "wrong passphrase" from "this file is damaged".
    ContainerCorrupt,
    /// `decrypt` refuses to overwrite a restricted file that already exists
    /// in the package directory.
    RestrictedFileExists {
        /// The file that would have been overwritten.
        path: std::path::PathBuf,
    },
    /// A `validation/regression/algo-v*/*.json` fixture is not valid JSON
    /// for the expected shape, or its `seed_hex` is malformed.
    RegressionFixtureParse {
        /// The fixture file that failed to parse.
        path: std::path::PathBuf,
    },
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
            Self::InvalidGeneratedAt { value } => write!(
                f,
                "generated_at must be YYYY-MM-DDTHH:MM:SSZ, got `{value}`"
            ),
            Self::PackageDirExists { path } => write!(
                f,
                "package directory already exists (refuse overwrite): {}",
                path.display()
            ),
            Self::Canonical(err) => write!(f, "{err}"),
            Self::Io(err) => write!(f, "package I/O failed: {err}"),
            Self::ListCsvParse { detail } => write!(f, "list.csv parse error: {detail}"),
            Self::ListCsvHeaderMismatch => write!(
                f,
                "list.csv header does not match config (expected config factor order)"
            ),
            Self::ListCsvNoDataRows => write!(f, "list.csv contains a header but no data rows"),
            Self::ListCsvColumnCount {
                row,
                expected,
                found,
            } => write!(
                f,
                "list.csv row {row} has {found} columns, expected {expected}"
            ),
            Self::ListCsvInvalidInteger { column, row } => write!(
                f,
                "list.csv row {row}: `{column}` is not a valid unsigned integer"
            ),
            Self::ChecksumsParse { line } => write!(
                f,
                "checksums.txt line {line}: expected `<hex>  <filename>` (GNU sha256sum text mode)"
            ),
            Self::ChecksumsInvalidDigest { line } => write!(
                f,
                "checksums.txt line {line}: digest must be 64 lowercase hex digits"
            ),
            Self::ChecksumsDuplicatePath { path } => {
                write!(f, "checksums.txt lists `{path}` more than once")
            }
            Self::ManifestJsonParse => write!(f, "manifest.unblinded.json is not valid JSON"),
            Self::BlindedManifestJsonParse => {
                write!(f, "manifest.blinded.json is not valid JSON")
            }
            Self::InvalidSeedHexLength => {
                write!(f, "invalid seed_hex length (expected 64 hex characters)")
            }
            Self::InvalidSeedHexEncoding => {
                write!(f, "invalid seed_hex encoding (expected lowercase hex)")
            }
            Self::EmptyPassphrase => write!(f, "passphrase must not be empty"),
            Self::KeyDerivationFailed => write!(f, "key derivation failed"),
            Self::DecryptionFailed => write!(
                f,
                "could not decrypt restricted.age (wrong passphrase, or this container does not belong to this package)"
            ),
            Self::ContainerCorrupt => write!(
                f,
                "restricted.age is corrupt or does not match checksums.txt"
            ),
            Self::RestrictedFileExists { path } => write!(
                f,
                "refuse to overwrite existing restricted file: {}",
                path.display()
            ),
            Self::RegressionFixtureParse { path } => write!(
                f,
                "regression fixture is not valid JSON for the expected shape: {}",
                path.display()
            ),
        }
    }
}

impl std::error::Error for PackageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::MissingStratumFactor { .. }
            | Self::InvalidGeneratedAt { .. }
            | Self::PackageDirExists { .. }
            | Self::ListCsvParse { .. }
            | Self::ListCsvHeaderMismatch
            | Self::ListCsvNoDataRows
            | Self::ListCsvColumnCount { .. }
            | Self::ListCsvInvalidInteger { .. }
            | Self::ChecksumsParse { .. }
            | Self::ChecksumsInvalidDigest { .. }
            | Self::ChecksumsDuplicatePath { .. }
            | Self::ManifestJsonParse
            | Self::BlindedManifestJsonParse
            | Self::InvalidSeedHexLength
            | Self::InvalidSeedHexEncoding
            | Self::EmptyPassphrase
            | Self::KeyDerivationFailed
            | Self::DecryptionFailed
            | Self::ContainerCorrupt
            | Self::RestrictedFileExists { .. }
            | Self::RegressionFixtureParse { .. } => None,
            Self::Canonical(err) => Some(err),
            Self::Io(err) => Some(err),
        }
    }
}
