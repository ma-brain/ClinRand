//! Command-flow integration test for the desktop Tauri host.
//!
//! Exercises the command API end-to-end **without a GUI**, in the same order
//! the operator drives the UI: `validate_config_json` → `preview_structure` →
//! `generate_package` → `verify_package`. This satisfies the Phase 7
//! "Done when" requirement for automated command-flow coverage and covers the
//! command-layer safety sequencing (Task 8's deferred concern). The access-log
//! gate is a UI-only control; its posture is documented in
//! `docs/security-posture.md` and `apps/desktop/README.md`.
//!
//! Two invariants are asserted directly:
//!
//! * the serialized `generate_package` outcome contains **no seed** material
//!   (AGENTS.md §4.9);
//! * a freshly generated package **verifies clean** (checksums + properties).

use std::path::Path;

use clinrand_desktop_lib::commands::config::{preview_structure, validate_config_json};
use clinrand_desktop_lib::commands::generate::generate_package;
use clinrand_desktop_lib::commands::verify::verify_package;

// Synthetic DEMO study: 1:1 stratified permuted block over two sites.
// list_length_per_stratum (8) × 2 strata = 16 records, 8 per arm.
const DEMO_CONFIG: &str = r#"{
    "schema_version": "1.0",
    "study_id": "DEMO-909",
    "protocol_version": "1.0",
    "arms": [
        { "code": "A", "label": "Active", "ratio": 1 },
        { "code": "P", "label": "Placebo", "ratio": 1 }
    ],
    "method": "stratified_block",
    "block": { "kind": "fixed", "size": 4 },
    "strata": [
        { "name": "site", "levels": ["001", "002"] }
    ],
    "list_length_per_stratum": 8,
    "numbering": { "kind": "global", "start": 10001, "width": 5 }
}"#;

#[test]
fn full_command_flow_validate_preview_generate_verify() {
    // 1. Validate — the config must be accepted with no errors.
    let validation = validate_config_json(DEMO_CONFIG.to_string(), false);
    assert!(validation.ok, "validation errors: {:?}", validation.errors);
    assert!(validation.errors.is_empty());

    // 2. Preview — blinded, config-derived structure. No arm assignments.
    let preview = preview_structure(DEMO_CONFIG.to_string()).expect("preview_structure");
    assert_eq!(preview.study_id, "DEMO-909");
    assert_eq!(preview.n_strata, 2);
    assert_eq!(preview.total_records, 16);
    assert_eq!(preview.arms.len(), 2);
    assert_eq!(preview.arms[0].total, 8);
    assert_eq!(preview.arms[1].total, 8);

    // 3. Generate — draw a seed, allocate, and write a package to a temp dir.
    let dir = std::env::temp_dir().join(format!(
        "clinrand-cmdflow-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&dir).expect("create temp out dir");

    let outcome = generate_package(
        DEMO_CONFIG.to_string(),
        dir.to_string_lossy().to_string(),
        "DEMO operator".to_string(),
        false,
        None,
    )
    .expect("generate_package");

    assert_eq!(outcome.record_count, preview.total_records);
    assert_eq!(outcome.list_sha256.len(), 64);
    assert!(Path::new(&outcome.package_dir).is_dir());

    // The seed lives only in `manifest.unblinded.json` on disk. The command
    // output must never carry it — confirm by serializing the outcome.
    let serialized = serde_json::to_string(&outcome).expect("serialize outcome");
    assert!(
        !serialized.contains("seed"),
        "generate outcome must not mention the seed: {serialized}"
    );

    // 4. Verify — the freshly generated package must round-trip clean.
    let verify = verify_package(outcome.package_dir.clone()).expect("verify_package");
    assert!(verify.ok, "verify outcome: {verify:?}");
    assert!(verify.checksums_ok, "checksums must pass");
    assert!(verify.properties_ok, "required properties must pass");
    assert!(verify.checksum_failures.is_empty());
    assert!(verify.property_failures.is_empty());

    std::fs::remove_dir_all(&dir).ok();
}
