//! Regression fixture loading and checking (plan §8.3,
//! `validation/regression/README.md`).
//!
//! A fixture pins `(config, seed) -> expected hashes` for a specific
//! `ALGO_VERSION` — proof that nothing changed since the last approved
//! version, never correctness (see `validation/README.md`).
//!
//! This module only loads and *compares*. It never calls `generate` itself:
//! [`check_regression_case`] takes an already-built `GeneratedList`, the
//! same convention [`crate::write_package`] uses for the same reason — the
//! thing that runs allocation is the caller (`clinrand-cli`'s
//! `validation-report`, or the `regression_fixtures` integration test), not
//! this crate.

use std::fs;
use std::path::Path;

use clinrand_core::{GeneratedList, StudyConfig};
use serde::Deserialize;

use crate::canonical::{config_sha256, sha256_hex};
use crate::error::PackageError;
use crate::list::render_list_csv;
use crate::stream::render_stream_csv;

/// One frozen `(config, seed) -> expected hashes` regression fixture.
#[derive(Clone, Debug)]
pub struct RegressionCase {
    /// Stable identifier, e.g. `algo-v1-simple-global`.
    pub case_id: String,
    /// The config this fixture was frozen against.
    pub config: StudyConfig,
    /// The seed this fixture was frozen against — synthetic, hardcoded,
    /// never a real study's seed (hard prohibition #10 permits synthetic
    /// seeds in fixtures; AGENTS.md §4.9 forbids a real one).
    pub seed: [u8; 32],
    /// Expected `config_sha256` (plan §6.4).
    pub expected_config_sha256: String,
    /// Expected SHA-256 of `render_list_csv` bytes.
    pub expected_list_sha256: String,
    /// Expected SHA-256 of `render_stream_csv` bytes.
    pub expected_stream_sha256: String,
    /// Expected `GeneratedList::records` length.
    pub expected_record_count: usize,
}

/// Outcome of checking one [`RegressionCase`] against a freshly generated list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegressionOutcome {
    /// The case this outcome belongs to.
    pub case_id: String,
    /// True when every check below matched.
    pub passed: bool,
    /// Human-readable mismatch details (empty when `passed`).
    pub failures: Vec<String>,
}

#[derive(Deserialize)]
struct RegressionFixtureWire {
    case_id: String,
    seed_hex: String,
    config: StudyConfig,
    expected: RegressionExpectedWire,
}

#[derive(Deserialize)]
struct RegressionExpectedWire {
    config_sha256: String,
    list_sha256: String,
    stream_sha256: String,
    record_count: usize,
}

/// Load every `*.json` regression fixture directly inside `dir`, sorted by
/// filename for a stable report order.
///
/// # Errors
///
/// Returns [`PackageError::Io`] if `dir` cannot be read, and
/// [`PackageError::RegressionFixtureParse`] if a fixture file is not valid
/// JSON for the expected shape or its `seed_hex` is malformed.
pub fn load_regression_cases(dir: &Path) -> Result<Vec<RegressionCase>, PackageError> {
    let mut paths: Vec<_> = fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    paths.sort();

    paths.iter().map(|path| load_case(path)).collect()
}

fn load_case(path: &Path) -> Result<RegressionCase, PackageError> {
    let parse_error = || PackageError::RegressionFixtureParse {
        path: path.to_path_buf(),
    };
    let text = fs::read_to_string(path)?;
    let wire: RegressionFixtureWire = serde_json::from_str(&text).map_err(|_| parse_error())?;
    let seed = decode_seed_hex(&wire.seed_hex).ok_or_else(parse_error)?;

    Ok(RegressionCase {
        case_id: wire.case_id,
        config: wire.config,
        seed,
        expected_config_sha256: wire.expected.config_sha256,
        expected_list_sha256: wire.expected.list_sha256,
        expected_stream_sha256: wire.expected.stream_sha256,
        expected_record_count: wire.expected.record_count,
    })
}

fn decode_seed_hex(hex: &str) -> Option<[u8; 32]> {
    if hex.len() != 64
        || !hex
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    {
        return None;
    }
    let mut seed = [0u8; 32];
    for (i, byte) in hex.as_bytes().chunks(2).enumerate() {
        let pair = std::str::from_utf8(byte).ok()?;
        seed[i] = u8::from_str_radix(pair, 16).ok()?;
    }
    Some(seed)
}

/// Compare a freshly generated `list` (from `case.config` + `case.seed`)
/// against `case`'s frozen expectations.
///
/// Checks `config_sha256`, `list_sha256`, `stream_sha256`, and record count
/// independently, collecting every mismatch rather than stopping at the
/// first — matches the reference tier's "report everything" behavior.
pub fn check_regression_case(case: &RegressionCase, list: &GeneratedList) -> RegressionOutcome {
    let mut failures = Vec::new();

    match config_sha256(&case.config) {
        Ok(hash) if hash == case.expected_config_sha256 => {}
        Ok(hash) => failures.push(format!(
            "config_sha256 mismatch: expected {}, got {hash}",
            case.expected_config_sha256
        )),
        Err(err) => failures.push(format!("config_sha256 failed: {err}")),
    }

    match render_list_csv(&case.config, list) {
        Ok(csv) => {
            let hash = sha256_hex(csv.as_bytes());
            if hash != case.expected_list_sha256 {
                failures.push(format!(
                    "list_sha256 mismatch: expected {}, got {hash}",
                    case.expected_list_sha256
                ));
            }
        }
        Err(err) => failures.push(format!("list.csv render failed: {err}")),
    }

    match render_stream_csv(&list.stream) {
        Ok(csv) => {
            let hash = sha256_hex(csv.as_bytes());
            if hash != case.expected_stream_sha256 {
                failures.push(format!(
                    "stream_sha256 mismatch: expected {}, got {hash}",
                    case.expected_stream_sha256
                ));
            }
        }
        Err(err) => failures.push(format!("stream.csv render failed: {err}")),
    }

    if list.records.len() != case.expected_record_count {
        failures.push(format!(
            "record_count mismatch: expected {}, got {}",
            case.expected_record_count,
            list.records.len()
        ));
    }

    RegressionOutcome {
        case_id: case.case_id.clone(),
        passed: failures.is_empty(),
        failures,
    }
}
