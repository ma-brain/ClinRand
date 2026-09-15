//! Package read helpers: `list.csv`, `checksums.txt`, unblinded manifest.

use std::collections::BTreeMap;

use clinrand_core::{AllocationRecord, StudyConfig};
use serde::Deserialize;

use crate::csv_util::parse_csv_rows;
use crate::error::PackageError;

/// Parsed `manifest.unblinded.json` fields needed for reproduce and verify.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnblindedManifest {
    /// Manifest schema version (e.g. `"1.0"`).
    pub schema_version: String,
    /// Study identifier from the config.
    pub study_id: String,
    /// Protocol version from the config.
    pub protocol_version: String,
    /// Generation timestamp (`YYYY-MM-DDTHH:MM:SSZ`).
    pub generated_at: String,
    /// Operator name recorded at generation time.
    pub operator: String,
    /// Embedded study configuration.
    pub config: StudyConfig,
    /// SHA-256 of canonical config JSON (lowercase hex).
    pub config_sha256: String,
    /// Lowercase hex encoding of the 32-byte seed (64 characters).
    pub seed_hex: String,
    /// SHA-256 of raw seed bytes (lowercase hex).
    pub seed_sha256: String,
    /// Allocation algorithm version at generation time.
    pub algo_version: u32,
    /// `clinrand-core` crate version at generation time.
    pub engine_version: String,
    /// Number of allocation records in the list.
    pub record_count: usize,
    /// SHA-256 of `list.csv` bytes (lowercase hex).
    pub list_sha256: String,
    /// SHA-256 of `stream.csv` bytes (lowercase hex).
    pub stream_sha256: String,
}

#[derive(Deserialize)]
struct UnblindedManifestWire {
    schema_version: String,
    study_id: String,
    protocol_version: String,
    generated_at: String,
    operator: String,
    config: StudyConfig,
    config_sha256: String,
    seed_hex: String,
    seed_sha256: String,
    algo_version: u32,
    engine_version: String,
    record_count: usize,
    list_sha256: String,
    stream_sha256: String,
}

/// Parse `list.csv` bytes into allocation records.
///
/// Column order follows config factor order, inverting
/// [`crate::render_list_csv`]. Stratum keys on each record match factor
/// names from `cfg.strata`.
///
/// # Errors
///
/// Rejects header mismatch, unknown columns, short rows, bad integers, and
/// malformed CSV. [`Display`](std::fmt::Display) never includes a seed.
pub fn parse_list_csv(cfg: &StudyConfig, csv: &str) -> Result<Vec<AllocationRecord>, PackageError> {
    let rows = parse_csv_rows(csv)?;
    let (header, data_rows) = rows.split_first().ok_or(PackageError::ListCsvNoDataRows)?;

    let factor_names: Vec<&str> = cfg.strata.iter().map(|f| f.name.as_str()).collect();
    let expected_header = list_csv_header_columns(&factor_names);
    if *header != expected_header {
        return Err(PackageError::ListCsvHeaderMismatch);
    }
    if data_rows.is_empty() {
        return Err(PackageError::ListCsvNoDataRows);
    }

    let mut records = Vec::with_capacity(data_rows.len());
    for (row_idx, row) in data_rows.iter().enumerate() {
        let row_num = row_idx + 2; // 1-based; row 1 is header
        if row.len() != expected_header.len() {
            return Err(PackageError::ListCsvColumnCount {
                row: row_num,
                expected: expected_header.len(),
                found: row.len(),
            });
        }

        let mut idx = 0;
        let randomization_number = row[idx].clone();
        idx += 1;

        let mut stratum = BTreeMap::new();
        for name in &factor_names {
            stratum.insert((*name).to_owned(), row[idx].clone());
            idx += 1;
        }

        let block_id = parse_u32_column("block_id", row_num, &row[idx])?;
        idx += 1;
        let block_size = parse_u32_column("block_size", row_num, &row[idx])?;
        idx += 1;
        let position_in_block = parse_u32_column("position_in_block", row_num, &row[idx])?;
        idx += 1;
        let arm_code = row[idx].clone();

        records.push(AllocationRecord {
            randomization_number,
            stratum,
            block_id,
            block_size,
            position_in_block,
            arm_code,
        });
    }

    Ok(records)
}

/// Parse GNU `sha256sum` text-mode `checksums.txt` lines (`<hex>  <name>`).
///
/// Keys are relative filenames; values are 64-character lowercase hex digests.
pub fn parse_checksums_txt(text: &str) -> Result<BTreeMap<String, String>, PackageError> {
    let mut map = BTreeMap::new();
    for (line_idx, line) in text.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        let line_no = line_idx + 1;
        let Some((hex, path)) = line.split_once("  ") else {
            return Err(PackageError::ChecksumsParse { line: line_no });
        };
        if hex.len() != 64
            || !hex
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        {
            return Err(PackageError::ChecksumsInvalidDigest { line: line_no });
        }
        if map.insert(path.to_owned(), hex.to_owned()).is_some() {
            return Err(PackageError::ChecksumsDuplicatePath {
                path: path.to_owned(),
            });
        }
    }
    Ok(map)
}

/// Parse `manifest.unblinded.json` into a typed struct.
///
/// # Errors
///
/// JSON parse failures and invalid `seed_hex` shape are reported without
/// echoing seed material in [`Display`](std::fmt::Display).
pub fn parse_unblinded_manifest(json: &str) -> Result<UnblindedManifest, PackageError> {
    let trimmed = json.trim_end_matches('\n');
    let wire: UnblindedManifestWire =
        serde_json::from_str(trimmed).map_err(|_| PackageError::ManifestJsonParse)?;
    validate_seed_hex(&wire.seed_hex)?;

    Ok(UnblindedManifest {
        schema_version: wire.schema_version,
        study_id: wire.study_id,
        protocol_version: wire.protocol_version,
        generated_at: wire.generated_at,
        operator: wire.operator,
        config: wire.config,
        config_sha256: wire.config_sha256,
        seed_hex: wire.seed_hex,
        seed_sha256: wire.seed_sha256,
        algo_version: wire.algo_version,
        engine_version: wire.engine_version,
        record_count: wire.record_count,
        list_sha256: wire.list_sha256,
        stream_sha256: wire.stream_sha256,
    })
}

#[derive(Deserialize)]
struct BlindedManifestWire {
    config: StudyConfig,
}

/// Extract study configuration from `manifest.blinded.json`.
///
/// Does not require or validate `seed_hex` (blinded manifest omits it).
///
/// # Errors
///
/// Returns [`PackageError::BlindedManifestJsonParse`] when JSON is invalid.
pub fn parse_blinded_manifest(json: &str) -> Result<StudyConfig, PackageError> {
    let trimmed = json.trim_end_matches('\n');
    let wire: BlindedManifestWire =
        serde_json::from_str(trimmed).map_err(|_| PackageError::BlindedManifestJsonParse)?;
    Ok(wire.config)
}

fn list_csv_header_columns(factor_names: &[&str]) -> Vec<String> {
    let mut header = vec!["randomization_number".to_owned()];
    header.extend(factor_names.iter().map(|name| (*name).to_owned()));
    header.extend(
        ["block_id", "block_size", "position_in_block", "arm_code"]
            .into_iter()
            .map(str::to_owned),
    );
    header
}

fn parse_u32_column(column: &str, row: usize, value: &str) -> Result<u32, PackageError> {
    value
        .parse::<u32>()
        .map_err(|_| PackageError::ListCsvInvalidInteger {
            column: column.to_owned(),
            row,
        })
}

fn validate_seed_hex(seed_hex: &str) -> Result<(), PackageError> {
    if seed_hex.len() != 64 {
        return Err(PackageError::InvalidSeedHexLength);
    }
    if !seed_hex
        .chars()
        .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    {
        return Err(PackageError::InvalidSeedHexEncoding);
    }
    Ok(())
}
