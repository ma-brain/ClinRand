//! Canonical JSON and config SHA-256 (plan §6.4).
//!
//! Hashing and canonicalization live in this crate, not in `clinrand-core`.
//! Plan §4 sketched `canonical_json` on core; that placement is overridden
//! here so `clinrand-core` stays free of hashing.

use clinrand_core::StudyConfig;
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Failure producing canonical JSON or `config_sha256`.
#[derive(Debug)]
pub enum CanonicalError {
    /// `StudyConfig` could not be serialized to a JSON value.
    Serialize(serde_json::Error),
    /// A JSON number that is not an integer (decimal or exponent form).
    NonIntegerNumber,
}

impl std::fmt::Display for CanonicalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Serialize(err) => write!(f, "failed to serialize study config: {err}"),
            Self::NonIntegerNumber => {
                f.write_str("canonical JSON numbers must be integers without decimal or exponent")
            }
        }
    }
}

impl std::error::Error for CanonicalError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Serialize(err) => Some(err),
            Self::NonIntegerNumber => None,
        }
    }
}

/// Compact canonical JSON of `cfg` (plan §6.4).
///
/// Contract-bound: serialize `cfg` to a JSON value, then canonicalize that
/// value. Object keys are sorted by UTF-8 byte order; arrays keep their
/// given order; nested objects are canonicalized recursively. No
/// insignificant whitespace, integers without decimal or exponent, strings
/// with minimal escaping, UTF-8, no trailing newline.
///
/// Changing this changes `config_sha256` and must be documented. It is not
/// an `ALGO_VERSION` bump (the allocation path is unchanged).
pub fn canonical_json(cfg: &StudyConfig) -> Result<String, CanonicalError> {
    let value = serde_json::to_value(cfg).map_err(CanonicalError::Serialize)?;
    let canonical = canonicalize_value(value)?;
    serde_json::to_string(&canonical).map_err(CanonicalError::Serialize)
}

/// SHA-256 of [`canonical_json`] as 64 lowercase hex characters, no `0x` prefix.
///
/// The digest is SHA-256 of the canonical JSON UTF-8 bytes.
pub fn config_sha256(cfg: &StudyConfig) -> Result<String, CanonicalError> {
    let digest = config_sha256_digest(cfg)?;
    Ok(to_hex_lowercase(&digest))
}

/// Raw 32-byte SHA-256 digest of the same canonical UTF-8 bytes as [`config_sha256`].
pub fn config_sha256_digest(cfg: &StudyConfig) -> Result<[u8; 32], CanonicalError> {
    let json = canonical_json(cfg)?;
    let digest = Sha256::digest(json.as_bytes());
    Ok(digest.into())
}

fn canonicalize_value(value: Value) -> Result<Value, CanonicalError> {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<(String, Value)> = map.into_iter().collect();
            entries.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
            let mut out = serde_json::Map::with_capacity(entries.len());
            for (key, child) in entries {
                out.insert(key, canonicalize_value(child)?);
            }
            Ok(Value::Object(out))
        }
        Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(canonicalize_value(item)?);
            }
            Ok(Value::Array(out))
        }
        Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                Ok(Value::Number(n))
            } else {
                Err(CanonicalError::NonIntegerNumber)
            }
        }
        other => Ok(other),
    }
}

const HEX: &[u8; 16] = b"0123456789abcdef";

fn to_hex_lowercase(bytes: &[u8; 32]) -> String {
    let mut out = String::with_capacity(64);
    for &byte in bytes {
        out.push(char::from(HEX[usize::from(byte >> 4)]));
        out.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    out
}
