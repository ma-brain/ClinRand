//! `write_package_encrypted` / `decrypt_package` — the `restricted.age`
//! container round trip, tamper detection, and `verify_package` behavior on
//! an encrypted package (plan §6.6).

use std::fs;

use clinrand_core::{generate, Arm, BlockScheme, Method, NumberingScheme, StudyConfig};
use clinrand_package::{
    decrypt_package, sha256_hex, verify_package, write_package, write_package_encrypted,
    PackageError, PackageMeta,
};

const PASSPHRASE: &str = "correct horse battery staple";

const RESTRICTED_FILES: &[&str] = &[
    "list.csv",
    "list.json",
    "manifest.unblinded.json",
    "stream.csv",
    "unblinded-report.html",
];

fn arms_ap() -> Vec<Arm> {
    vec![
        Arm {
            code: "A".into(),
            label: "Active".into(),
            ratio: 1,
        },
        Arm {
            code: "P".into(),
            label: "Placebo".into(),
            ratio: 1,
        },
    ]
}

fn demo_cfg(study_id: &str) -> StudyConfig {
    StudyConfig {
        schema_version: "1.0".into(),
        study_id: study_id.into(),
        protocol_version: "1.0".into(),
        arms: arms_ap(),
        method: Method::PermutedBlock {
            block: BlockScheme::Fixed { size: 2 },
        },
        strata: vec![],
        list_length_per_stratum: 2,
        numbering: NumberingScheme::Global {
            start: 10001,
            width: 5,
        },
    }
}

fn meta() -> PackageMeta {
    PackageMeta {
        operator: "Encrypt Test Operator".into(),
        generated_at: "2026-09-15T14:42:10Z".into(),
    }
}

fn demo_seed() -> [u8; 32] {
    let mut seed = [0u8; 32];
    seed[0] = 0x02;
    seed[31] = 0xcd;
    seed
}

#[test]
fn round_trip_matches_plaintext_write_package_byte_for_byte() {
    let cfg = demo_cfg("DEMO-810");
    let list = generate(&cfg, demo_seed()).expect("generate");

    let plain_tmp = tempfile::tempdir().expect("tempdir");
    let plain_dir = write_package(plain_tmp.path(), &cfg, &list, &demo_seed(), &meta())
        .expect("write plaintext package");

    let enc_tmp = tempfile::tempdir().expect("tempdir");
    let enc_dir = write_package_encrypted(
        enc_tmp.path(),
        &cfg,
        &list,
        &demo_seed(),
        &meta(),
        PASSPHRASE,
    )
    .expect("write encrypted package");

    // The 5 restricted files must not exist as plaintext; `restricted.age`
    // must exist instead.
    for name in RESTRICTED_FILES {
        assert!(
            !enc_dir.join(name).exists(),
            "{name} must not be written as plaintext when encrypted"
        );
    }
    assert!(enc_dir.join("restricted.age").is_file());

    decrypt_package(&enc_dir, PASSPHRASE).expect("decrypt");

    for name in RESTRICTED_FILES {
        let plain_bytes = fs::read(plain_dir.join(name)).expect("read plaintext file");
        let decrypted_bytes = fs::read(enc_dir.join(name)).expect("read decrypted file");
        assert_eq!(
            plain_bytes, decrypted_bytes,
            "{name} must round-trip byte-identical through encrypt/decrypt"
        );
    }
}

#[test]
fn wrong_passphrase_is_rejected_without_writing_anything() {
    let cfg = demo_cfg("DEMO-811");
    let list = generate(&cfg, demo_seed()).expect("generate");
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = write_package_encrypted(tmp.path(), &cfg, &list, &demo_seed(), &meta(), PASSPHRASE)
        .expect("write encrypted package");

    let err = decrypt_package(&dir, "definitely the wrong passphrase").unwrap_err();
    assert!(matches!(err, PackageError::DecryptionFailed));

    for name in RESTRICTED_FILES {
        assert!(
            !dir.join(name).exists(),
            "a failed decrypt must not leave {name} on disk"
        );
    }
}

#[test]
fn tampered_container_bytes_are_caught_by_checksum_before_aead() {
    let cfg = demo_cfg("DEMO-812");
    let list = generate(&cfg, demo_seed()).expect("generate");
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = write_package_encrypted(tmp.path(), &cfg, &list, &demo_seed(), &meta(), PASSPHRASE)
        .expect("write encrypted package");

    let container_path = dir.join("restricted.age");
    let mut bytes = fs::read(&container_path).expect("read restricted.age");
    let idx = bytes.len().saturating_sub(2);
    bytes[idx] ^= 0x01;
    fs::write(&container_path, &bytes).expect("corrupt restricted.age");

    // checksums.txt was not updated, so the corruption is caught before AEAD
    // decryption is even attempted.
    let err = decrypt_package(&dir, PASSPHRASE).unwrap_err();
    assert!(matches!(err, PackageError::ContainerCorrupt));
}

#[test]
fn tampered_ciphertext_with_matching_checksum_fails_aead_authentication() {
    let cfg = demo_cfg("DEMO-813");
    let list = generate(&cfg, demo_seed()).expect("generate");
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = write_package_encrypted(tmp.path(), &cfg, &list, &demo_seed(), &meta(), PASSPHRASE)
        .expect("write encrypted package");

    let container_path = dir.join("restricted.age");
    let mut bytes = fs::read(&container_path).expect("read restricted.age");
    let idx = bytes.len().saturating_sub(2);
    bytes[idx] ^= 0x01;
    fs::write(&container_path, &bytes).expect("corrupt restricted.age");

    // Recompute checksums.txt's restricted.age line so the flipped bytes
    // pass the checksum gate; AEAD authentication must still refuse them.
    let new_hash = sha256_hex(&bytes);
    let checksums_path = dir.join("checksums.txt");
    let checksums = fs::read_to_string(&checksums_path).expect("read checksums.txt");
    let mut lines: Vec<String> = checksums
        .lines()
        .map(|line| match line.split_once("  ") {
            Some((_, "restricted.age")) => format!("{new_hash}  restricted.age"),
            _ => line.to_owned(),
        })
        .collect();
    lines.sort();
    let mut updated = lines.join("\n");
    updated.push('\n');
    fs::write(&checksums_path, updated).expect("write checksums.txt");

    let err = decrypt_package(&dir, PASSPHRASE).unwrap_err();
    assert!(matches!(err, PackageError::DecryptionFailed));
}

#[test]
fn empty_passphrase_is_rejected_on_encrypt_and_decrypt() {
    let cfg = demo_cfg("DEMO-814");
    let list = generate(&cfg, demo_seed()).expect("generate");
    let tmp = tempfile::tempdir().expect("tempdir");

    let err =
        write_package_encrypted(tmp.path(), &cfg, &list, &demo_seed(), &meta(), "   ").unwrap_err();
    assert!(matches!(err, PackageError::EmptyPassphrase));

    let dir = write_package_encrypted(tmp.path(), &cfg, &list, &demo_seed(), &meta(), PASSPHRASE)
        .expect("write encrypted package");
    let err = decrypt_package(&dir, "").unwrap_err();
    assert!(matches!(err, PackageError::EmptyPassphrase));
}

#[test]
fn decrypt_refuses_to_overwrite_an_existing_restricted_file() {
    let cfg = demo_cfg("DEMO-815");
    let list = generate(&cfg, demo_seed()).expect("generate");
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = write_package_encrypted(tmp.path(), &cfg, &list, &demo_seed(), &meta(), PASSPHRASE)
        .expect("write encrypted package");

    fs::write(dir.join("list.csv"), b"pre-existing").expect("plant conflicting file");

    let err = decrypt_package(&dir, PASSPHRASE).unwrap_err();
    assert!(matches!(err, PackageError::RestrictedFileExists { .. }));
    // The plant is untouched; nothing else was decrypted onto disk either.
    assert_eq!(
        fs::read(dir.join("list.csv")).expect("read planted file"),
        b"pre-existing"
    );
    assert!(!dir.join("stream.csv").exists());
}

#[test]
fn verify_package_skips_properties_until_decrypted_then_checks_them() {
    let cfg = demo_cfg("DEMO-816");
    let list = generate(&cfg, demo_seed()).expect("generate");
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = write_package_encrypted(tmp.path(), &cfg, &list, &demo_seed(), &meta(), PASSPHRASE)
        .expect("write encrypted package");

    let before = verify_package(&dir).expect("verify encrypted package");
    assert!(before.checksums_ok, "report: {before:?}");
    assert!(!before.properties_checked);
    assert!(before.properties_ok, "trivially true when not checked");
    assert!(before.ok());

    decrypt_package(&dir, PASSPHRASE).expect("decrypt");

    let after = verify_package(&dir).expect("verify decrypted package");
    assert!(after.properties_checked);
    assert!(after.ok(), "report: {after:?}");
}
