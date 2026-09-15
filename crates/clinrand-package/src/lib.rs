//! Output package reading, writing, hashing, and reports.
//!
//! Allocation logic lives in `clinrand-core` and must not be implemented here.

#![forbid(unsafe_code)]
#![deny(clippy::all)]

mod canonical;
mod csv_util;
mod encrypt;
mod error;
mod list;
mod manifest;
mod qc;
mod read;
mod regression;
mod report;
mod stream;
mod verify;
mod write;

pub use canonical::{
    canonical_json, canonical_json_value, config_sha256, config_sha256_digest, sha256_hex,
    CanonicalError,
};
pub use clinrand_core::{ALGO_VERSION, ENGINE_VERSION, RNG_CRATE_VERSION};
pub use encrypt::{decrypt_package, write_package_encrypted};
pub use error::PackageError;
pub use list::{render_list_csv, render_list_json};
pub use manifest::{
    build_manifests, seed_hex, seed_sha256, ManifestPair, PackageMeta, MANIFEST_SCHEMA_VERSION,
};
pub use qc::render_qc_r;
pub use read::{
    parse_blinded_manifest, parse_checksums_txt, parse_list_csv, parse_unblinded_manifest,
    UnblindedManifest,
};
pub use regression::{
    check_regression_case, load_regression_cases, RegressionCase, RegressionOutcome,
};
pub use report::{render_generation_report, render_unblinded_report, ReportFileHashes};
pub use stream::render_stream_csv;
pub use verify::{verify_package, VerifyReport};
pub use write::{compact_generated_at, write_package};

#[cfg(test)]
mod tests {
    use clinrand_core::StudyConfig;

    use super::{canonical_json, canonical_json_value, config_sha256, config_sha256_digest};

    /// Plan §5.1 shape; object keys in typical config-file order.
    const KEYS_SCHEMA_ORDER: &str = r#"{
  "schema_version": "1.0",
  "study_id": "DEMO-201",
  "protocol_version": "2.1",
  "arms": [
    { "code": "A", "label": "Investigational product 50 mg", "ratio": 2 },
    { "code": "P", "label": "Placebo", "ratio": 1 }
  ],
  "method": "stratified_block",
  "block": { "kind": "variable", "sizes": [6, 9] },
  "strata": [
    { "name": "site", "levels": ["001", "002", "003"] },
    { "name": "agegrp", "levels": ["LT65", "GE65"] }
  ],
  "list_length_per_stratum": 36,
  "numbering": { "kind": "global", "start": 10001, "width": 5 }
}"#;

    /// Same logical config; object keys reversed at every nesting level.
    const KEYS_REVERSED: &str = r#"{
  "numbering": { "width": 5, "start": 10001, "kind": "global" },
  "list_length_per_stratum": 36,
  "strata": [
    { "levels": ["001", "002", "003"], "name": "site" },
    { "levels": ["LT65", "GE65"], "name": "agegrp" }
  ],
  "block": { "sizes": [6, 9], "kind": "variable" },
  "method": "stratified_block",
  "arms": [
    { "ratio": 2, "label": "Investigational product 50 mg", "code": "A" },
    { "ratio": 1, "label": "Placebo", "code": "P" }
  ],
  "protocol_version": "2.1",
  "study_id": "DEMO-201",
  "schema_version": "1.0"
}"#;

    /// Lexicographic compact form of the DEMO-201 config (UTF-8 key order).
    const EXPECTED_CANONICAL: &str = concat!(
        r#"{"arms":[{"code":"A","label":"Investigational product 50 mg","ratio":2},"#,
        r#"{"code":"P","label":"Placebo","ratio":1}],"block":{"kind":"variable","sizes":[6,9]},"#,
        r#""list_length_per_stratum":36,"method":"stratified_block","#,
        r#""numbering":{"kind":"global","start":10001,"width":5},"protocol_version":"2.1","#,
        r#""schema_version":"1.0","strata":[{"levels":["001","002","003"],"name":"site"},"#,
        r#"{"levels":["LT65","GE65"],"name":"agegrp"}],"study_id":"DEMO-201"}"#
    );

    /// SHA-256 (lowercase hex) of `EXPECTED_CANONICAL` UTF-8 bytes.
    const EXPECTED_SHA256_HEX: &str =
        "1364cea5ed27222f7d53130d10dbe7d94eb70b79e5fe0e2d007d2aa9979f01be";

    fn parse(json: &str) -> StudyConfig {
        serde_json::from_str(json).expect("fixture is a valid StudyConfig")
    }

    #[test]
    fn reexports_core_algo_version() {
        assert_eq!(clinrand_core::ALGO_VERSION, 1);
    }

    #[test]
    fn same_config_different_key_orders_are_byte_identical() {
        let a = parse(KEYS_SCHEMA_ORDER);
        let b = parse(KEYS_REVERSED);
        assert_eq!(a, b, "fixtures must be the same logical config");

        let json_a = canonical_json(&a).expect("canonical JSON");
        let json_b = canonical_json(&b).expect("canonical JSON");
        assert_eq!(
            json_a.as_bytes(),
            json_b.as_bytes(),
            "canonical JSON must be byte-identical"
        );

        let hash_a = config_sha256(&a).expect("config SHA-256");
        let hash_b = config_sha256(&b).expect("config SHA-256");
        assert_eq!(hash_a, hash_b);
        assert_eq!(
            config_sha256_digest(&a).expect("digest"),
            config_sha256_digest(&b).expect("digest")
        );
    }

    #[test]
    fn canonical_json_sorts_object_keys_and_keeps_array_order() {
        let cfg = parse(KEYS_REVERSED);
        let json = canonical_json(&cfg).expect("canonical JSON");
        assert_eq!(json, EXPECTED_CANONICAL);
        assert!(
            !json.ends_with('\n'),
            "canonical JSON must not have a trailing newline"
        );
        assert!(
            !json.contains(['\t', '\n', '\r']),
            "canonical JSON must not contain tabs or line breaks"
        );
        assert!(
            json.contains(r#""sizes":[6,9]"#),
            "array element order must be preserved"
        );
        assert!(
            json.contains(r#""levels":["001","002","003"]"#),
            "stratum level order must be preserved"
        );
    }

    #[test]
    fn config_sha256_is_lowercase_hex_of_canonical_bytes() {
        let cfg = parse(KEYS_SCHEMA_ORDER);
        let hex = config_sha256(&cfg).expect("config SHA-256");
        assert_eq!(hex, EXPECTED_SHA256_HEX);
        assert_eq!(hex.len(), 64);
        assert!(
            hex.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "digest must be lowercase hex with no 0x prefix"
        );
        assert!(!hex.starts_with("0x"));

        let digest = config_sha256_digest(&cfg).expect("digest");
        assert_eq!(digest.len(), 32);
        let from_digest: String = digest.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(from_digest, hex);
    }

    #[test]
    fn canonical_json_value_of_parsed_demo_201_matches_typed_config() {
        let cfg = parse(KEYS_SCHEMA_ORDER);
        let typed = canonical_json(&cfg).expect("canonical JSON of typed config");

        let parsed: serde_json::Value =
            serde_json::from_str(KEYS_SCHEMA_ORDER).expect("DEMO-201 JSON");
        let from_value = canonical_json_value(&parsed).expect("canonical JSON of parsed value");
        assert_eq!(
            typed, from_value,
            "canonical_json_value of parsed DEMO-201 must match canonical_json of StudyConfig"
        );

        let reversed: serde_json::Value =
            serde_json::from_str(KEYS_REVERSED).expect("reversed DEMO-201 JSON");
        let from_reversed =
            canonical_json_value(&reversed).expect("canonical JSON of reversed value");
        assert_eq!(typed, from_reversed);
        assert_eq!(from_value, EXPECTED_CANONICAL);
    }
}
