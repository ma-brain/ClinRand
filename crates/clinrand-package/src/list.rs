//! list.csv and list.json renderers (plan §6.1).
//!
//! Encoding: UTF-8, LF line endings, no BOM. The returned string ends with
//! exactly one trailing `\n` after the last row (or after the header when
//! there are no records). CSV fields are quoted only when they contain a
//! comma, double-quote, or newline (minimal RFC 4180-style).
//!
//! `list.json` shape: `{"records":[{...},...]}` — one object per allocation.
//! Object keys match `list.csv` columns and are emitted in that same order
//! (config factor order for stratum fields). Compact JSON, no insignificant
//! whitespace, trailing `\n`. Byte-stable for the same inputs. No seed.

use clinrand_core::{AllocationRecord, GeneratedList, StudyConfig};

use crate::csv_util::escape_csv_field;
use crate::error::PackageError;

/// Render `list.csv` bytes as a UTF-8 string (plan §6.1).
///
/// Pure function of `(cfg, list)`. Stratum columns follow **config factor
/// order** (`cfg.strata`); empty strata yields no stratum columns between
/// `randomization_number` and `block_id`. Does not include the seed.
pub fn render_list_csv(cfg: &StudyConfig, list: &GeneratedList) -> Result<String, PackageError> {
    let factor_names: Vec<&str> = cfg.strata.iter().map(|f| f.name.as_str()).collect();
    let mut out = String::new();

    push_list_header_csv(&mut out, &factor_names);
    out.push('\n');

    for record in &list.records {
        push_list_row_csv(&mut out, &factor_names, record)?;
        out.push('\n');
    }

    Ok(out)
}

/// Render `list.json` as a compact UTF-8 string.
///
/// See module docs for shape and trailing-newline policy.
pub fn render_list_json(cfg: &StudyConfig, list: &GeneratedList) -> Result<String, PackageError> {
    let factor_names: Vec<&str> = cfg.strata.iter().map(|f| f.name.as_str()).collect();
    // Manual emission keeps key order identical to list.csv columns.
    // serde_json::Map sorts keys unless the preserve_order feature is on.
    let mut out = String::from("{\"records\":[");
    for (i, record) in list.records.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        push_list_record_json(&mut out, &factor_names, record)?;
    }
    out.push_str("]}\n");
    Ok(out)
}

fn push_list_header_csv(out: &mut String, factor_names: &[&str]) {
    out.push_str("randomization_number");
    for name in factor_names {
        out.push(',');
        out.push_str(&escape_csv_field(name));
    }
    out.push_str(",block_id,block_size,position_in_block,arm_code");
}

fn push_list_row_csv(
    out: &mut String,
    factor_names: &[&str],
    record: &AllocationRecord,
) -> Result<(), PackageError> {
    out.push_str(&escape_csv_field(&record.randomization_number));
    for name in factor_names {
        let level = stratum_level(record, name)?;
        out.push(',');
        out.push_str(&escape_csv_field(level));
    }
    out.push(',');
    out.push_str(&record.block_id.to_string());
    out.push(',');
    out.push_str(&record.block_size.to_string());
    out.push(',');
    out.push_str(&record.position_in_block.to_string());
    out.push(',');
    out.push_str(&escape_csv_field(&record.arm_code));
    Ok(())
}

fn push_list_record_json(
    out: &mut String,
    factor_names: &[&str],
    record: &AllocationRecord,
) -> Result<(), PackageError> {
    out.push('{');
    push_json_string_entry(out, "randomization_number", &record.randomization_number);
    for name in factor_names {
        out.push(',');
        push_json_string_entry(out, name, stratum_level(record, name)?);
    }
    out.push(',');
    push_json_u32_entry(out, "block_id", record.block_id);
    out.push(',');
    push_json_u32_entry(out, "block_size", record.block_size);
    out.push(',');
    push_json_u32_entry(out, "position_in_block", record.position_in_block);
    out.push(',');
    push_json_string_entry(out, "arm_code", &record.arm_code);
    out.push('}');
    Ok(())
}

fn stratum_level<'a>(record: &'a AllocationRecord, factor: &str) -> Result<&'a str, PackageError> {
    record
        .stratum
        .get(factor)
        .map(String::as_str)
        .ok_or_else(|| PackageError::MissingStratumFactor {
            factor: factor.to_owned(),
            randomization_number: record.randomization_number.clone(),
        })
}

fn push_json_string_entry(out: &mut String, key: &str, value: &str) {
    out.push('"');
    push_json_escaped(out, key);
    out.push_str("\":\"");
    push_json_escaped(out, value);
    out.push('"');
}

fn push_json_u32_entry(out: &mut String, key: &str, value: u32) {
    out.push('"');
    push_json_escaped(out, key);
    out.push_str("\":");
    out.push_str(&value.to_string());
}

/// Minimal JSON string escaping (RFC 8259): control chars, `"`, and `\`.
fn push_json_escaped(out: &mut String, s: &str) {
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                use std::fmt::Write as _;
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
}
