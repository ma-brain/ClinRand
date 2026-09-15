//! Blinded and unblinded package manifests (plan §6.2–§6.3).
//!
//! Manifest file bytes are compact canonical JSON (plan §6.4 key order) plus
//! a trailing `\n`. `list_sha256` / `stream_sha256` digest the exact UTF-8
//! bytes from [`crate::render_list_csv`] / [`crate::render_stream_csv`].

use clinrand_core::{GeneratedList, StudyConfig, ALGO_VERSION, ENGINE_VERSION, RNG_CRATE_VERSION};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

use crate::canonical::{
    canonical_json_value, config_sha256, sha256_hex, to_hex_lowercase, CanonicalError,
};
use crate::error::PackageError;
use crate::list::render_list_csv;
use crate::stream::render_stream_csv;

/// Manifest schema version written into both package manifests.
pub const MANIFEST_SCHEMA_VERSION: &str = "1.0";

/// RNG identity recorded in manifests (plan §6.2). Must match the pinned
/// `rand_chacha` used by `clinrand-core`.
const RNG_ALGORITHM: &str = "ChaCha20";
const RNG_CRATE: &str = "rand_chacha";

/// Caller-supplied metadata that is not derived from `(config, seed)`.
///
/// `generated_at` must be `YYYY-MM-DDTHH:MM:SSZ` (e.g. `2026-09-15T14:42:10Z`).
/// The package crate does not read the clock.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageMeta {
    /// Operator name written to both manifests.
    pub operator: String,
    /// Pre-formatted generation timestamp (`YYYY-MM-DDTHH:MM:SSZ`).
    pub generated_at: String,
}

impl PackageMeta {
    /// Construct metadata after validating `generated_at` as `YYYY-MM-DDTHH:MM:SSZ`.
    ///
    /// # Errors
    ///
    /// Returns [`PackageError::InvalidGeneratedAt`] when the timestamp shape is wrong.
    pub fn new(
        operator: impl Into<String>,
        generated_at: impl Into<String>,
    ) -> Result<Self, PackageError> {
        let generated_at = generated_at.into();
        validate_generated_at(&generated_at)?;
        Ok(Self {
            operator: operator.into(),
            generated_at,
        })
    }
}

/// Reject timestamps that are not exactly `YYYY-MM-DDTHH:MM:SSZ`.
fn validate_generated_at(generated_at: &str) -> Result<(), PackageError> {
    let b = generated_at.as_bytes();
    let ok = b.len() == 20
        && b[0].is_ascii_digit()
        && b[1].is_ascii_digit()
        && b[2].is_ascii_digit()
        && b[3].is_ascii_digit()
        && b[4] == b'-'
        && b[5].is_ascii_digit()
        && b[6].is_ascii_digit()
        && b[7] == b'-'
        && b[8].is_ascii_digit()
        && b[9].is_ascii_digit()
        && b[10] == b'T'
        && b[11].is_ascii_digit()
        && b[12].is_ascii_digit()
        && b[13] == b':'
        && b[14].is_ascii_digit()
        && b[15].is_ascii_digit()
        && b[16] == b':'
        && b[17].is_ascii_digit()
        && b[18].is_ascii_digit()
        && b[19] == b'Z';
    if ok {
        Ok(())
    } else {
        Err(PackageError::InvalidGeneratedAt {
            value: generated_at.to_owned(),
        })
    }
}

/// In-memory blinded and unblinded manifest file contents.
///
/// Each string is compact canonical JSON plus a trailing `\n`, ready to write
/// as `manifest.blinded.json` / `manifest.unblinded.json`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManifestPair {
    /// `manifest.unblinded.json` bytes (includes `seed_hex`).
    pub unblinded: String,
    /// `manifest.blinded.json` bytes (`seed_hex` key omitted entirely).
    pub blinded: String,
}

/// Build both package manifests from a generated list and seed.
///
/// Pure of filesystem and clock: `meta.generated_at` and `meta.operator` come
/// from the caller. Does not write files.
///
/// # Errors
///
/// Propagates list/stream render failures and canonicalization / config hash
/// failures. Rejects invalid `generated_at`. Error [`Display`](std::fmt::Display)
/// never includes the seed.
pub fn build_manifests(
    cfg: &StudyConfig,
    list: &GeneratedList,
    seed: &[u8; 32],
    meta: &PackageMeta,
) -> Result<ManifestPair, PackageError> {
    validate_generated_at(&meta.generated_at)?;

    let list_csv = render_list_csv(cfg, list)?;
    let stream_csv = render_stream_csv(&list.stream)?;

    let list_sha256 = sha256_hex(list_csv.as_bytes());
    let stream_sha256 = sha256_hex(stream_csv.as_bytes());
    let config_digest = config_sha256(cfg).map_err(PackageError::from)?;
    let seed_digest = seed_sha256(seed);
    let seed_hex_str = seed_hex(seed);

    let config_value = serde_json::to_value(cfg)
        .map_err(|err| PackageError::from(CanonicalError::Serialize(err)))?;

    let common = common_manifest_fields(
        cfg,
        meta,
        config_value,
        &config_digest,
        &seed_digest,
        ENGINE_VERSION,
        list.records.len(),
        &list_sha256,
        &stream_sha256,
    );

    let mut unblinded_map = common.clone();
    unblinded_map.insert("seed_hex".into(), Value::String(seed_hex_str));

    let blinded_map = common;
    debug_assert!(
        !blinded_map.contains_key("seed_hex"),
        "blinded manifest must not contain seed_hex"
    );

    Ok(ManifestPair {
        unblinded: encode_manifest_file(&Value::Object(unblinded_map))?,
        blinded: encode_manifest_file(&Value::Object(blinded_map))?,
    })
}

/// SHA-256 of the raw 32 seed bytes as 64 lowercase hex characters.
///
/// Contract: digests the seed bytes themselves, not the hex string.
pub fn seed_sha256(seed: &[u8; 32]) -> String {
    let digest = Sha256::digest(seed);
    to_hex_lowercase(&digest.into())
}

/// Lowercase hex encoding of the 32 raw seed bytes (64 characters).
pub fn seed_hex(seed: &[u8; 32]) -> String {
    to_hex_lowercase(seed)
}

#[allow(clippy::too_many_arguments)]
fn common_manifest_fields(
    cfg: &StudyConfig,
    meta: &PackageMeta,
    config_value: Value,
    config_digest: &str,
    seed_digest: &str,
    engine_version: &str,
    record_count: usize,
    list_sha256: &str,
    stream_sha256: &str,
) -> Map<String, Value> {
    let mut map = Map::new();
    map.insert(
        "schema_version".into(),
        Value::String(MANIFEST_SCHEMA_VERSION.into()),
    );
    map.insert("study_id".into(), Value::String(cfg.study_id.clone()));
    map.insert(
        "protocol_version".into(),
        Value::String(cfg.protocol_version.clone()),
    );
    map.insert(
        "generated_at".into(),
        Value::String(meta.generated_at.clone()),
    );
    map.insert("operator".into(), Value::String(meta.operator.clone()));
    map.insert("config".into(), config_value);
    map.insert(
        "config_sha256".into(),
        Value::String(config_digest.to_owned()),
    );
    map.insert("seed_sha256".into(), Value::String(seed_digest.to_owned()));
    map.insert(
        "rng".into(),
        json!({
            "algorithm": RNG_ALGORITHM,
            "crate": RNG_CRATE,
            "crate_version": RNG_CRATE_VERSION,
        }),
    );
    map.insert(
        "engine_version".into(),
        Value::String(engine_version.to_owned()),
    );
    map.insert("algo_version".into(), Value::from(ALGO_VERSION));
    map.insert("record_count".into(), Value::from(record_count));
    map.insert("list_sha256".into(), Value::String(list_sha256.to_owned()));
    map.insert(
        "stream_sha256".into(),
        Value::String(stream_sha256.to_owned()),
    );
    map
}

fn encode_manifest_file(value: &Value) -> Result<String, PackageError> {
    let mut out = canonical_json_value(value).map_err(PackageError::from)?;
    out.push('\n');
    Ok(out)
}
