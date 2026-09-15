//! `restricted.age` container: encryption at rest for restricted package
//! files (plan §6.6, `docs/decisions/0007-restricted-container-format.md`).
//!
//! Wraps `list.csv`, `list.json`, `manifest.unblinded.json`, `stream.csv`,
//! and `unblinded-report.html` in a single AEAD-encrypted container using
//! XChaCha20-Poly1305 with an Argon2id-derived key. [`write_package_encrypted`]
//! never writes those five files as plaintext. [`decrypt_package`] reverses
//! it **in place**, so downstream tooling (`qc.R`, `verify_package`, the
//! unblinded report viewer) sees exactly the plaintext files a non-encrypted
//! run would have produced.
//!
//! Container layout (all integers little-endian):
//!
//! ```text
//! magic       4 bytes   b"CRV1"
//! kdf_m_cost  u32       Argon2id memory cost (KiB)
//! kdf_t_cost  u32       Argon2id iterations
//! kdf_p_cost  u32       Argon2id parallelism
//! salt        16 bytes
//! nonce       24 bytes  XChaCha20 nonce
//! ciphertext  remainder AEAD output (includes the 16-byte Poly1305 tag)
//! ```
//!
//! The plaintext wrapped by the AEAD is a minimal multi-file archive: for
//! each restricted file, sorted by name, `u16 name_len | name (UTF-8) | u64
//! content_len | content`. Associated data binds the container to its
//! package (`study_id` + `list_sha256`) so a `restricted.age` cannot be
//! silently swapped between package directories and still decrypt.
//!
//! This format is independent of `ALGO_VERSION` (AGENTS.md §3), which governs
//! allocation determinism only — encryption is a package/file-format
//! concern, not an allocation concern.

use std::fs;
use std::path::{Path, PathBuf};

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, Payload};
use chacha20poly1305::{Key as ChachaKey, KeyInit, XChaCha20Poly1305, XNonce};
use zeroize::Zeroizing;

use clinrand_core::{GeneratedList, StudyConfig};

use crate::canonical::sha256_hex;
use crate::error::PackageError;
use crate::manifest::PackageMeta;
use crate::read::parse_checksums_txt;
use crate::write::{render_checksums_txt, render_package, write_file};

/// Container format magic (`CRV1` — ClinRand restricted container, v1).
const MAGIC: &[u8; 4] = b"CRV1";
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 24;
const KEY_LEN: usize = 32;
const HEADER_LEN: usize = 4 + 4 + 4 + 4 + SALT_LEN + NONCE_LEN;

/// Argon2id parameters used for new containers. Deliberately expensive
/// (~1-2s on a laptop): this protects clinical trial allocation data, not a
/// login form. Stored in the container header so future containers can tune
/// cost without a new magic/version.
const ARGON2_M_COST: u32 = 131_072; // 128 MiB
const ARGON2_T_COST: u32 = 3;
const ARGON2_P_COST: u32 = 4;

/// Restricted filenames, sorted — the archive entry order and the set
/// [`decrypt_package`] refuses to overwrite.
const RESTRICTED_FILES: &[&str] = &[
    "list.csv",
    "list.json",
    "manifest.unblinded.json",
    "stream.csv",
    "unblinded-report.html",
];

/// Filenames written to disk by [`write_package_encrypted`] (excluding
/// `checksums.txt`), sorted — also the `checksums.txt` line order.
const ENCRYPTED_PACKAGE_FILES: &[&str] = &[
    "generation-report.html",
    "manifest.blinded.json",
    "qc.R",
    "restricted.age",
];

/// Write a ClinRand output package with restricted files encrypted at rest.
///
/// Renders the same content as [`crate::write_package`] but writes only the
/// 4 non-restricted files plus `restricted.age` — a single AEAD-encrypted
/// container holding `list.csv`, `list.json`, `manifest.unblinded.json`,
/// `stream.csv`, and `unblinded-report.html`. No plaintext restricted file is
/// ever written to disk.
///
/// # Errors
///
/// Returns [`PackageError::EmptyPassphrase`] for an empty or whitespace-only
/// passphrase, [`PackageError::PackageDirExists`] if the target directory
/// already exists, and propagates render / manifest / I/O / key-derivation
/// failures. [`PackageError`] [`Display`](std::fmt::Display) never includes
/// the seed or the passphrase.
pub fn write_package_encrypted(
    out_dir: &Path,
    cfg: &StudyConfig,
    list: &GeneratedList,
    seed: &[u8; 32],
    meta: &PackageMeta,
    passphrase: &str,
) -> Result<PathBuf, PackageError> {
    if passphrase.trim().is_empty() {
        return Err(PackageError::EmptyPassphrase);
    }

    let rendered = render_package(cfg, list, seed, meta)?;

    let archive = build_archive(&[
        ("list.csv", rendered.list_csv.as_bytes()),
        ("list.json", rendered.list_json.as_bytes()),
        (
            "manifest.unblinded.json",
            rendered.manifests.unblinded.as_bytes(),
        ),
        ("stream.csv", rendered.stream_csv.as_bytes()),
        (
            "unblinded-report.html",
            rendered.unblinded_report.as_bytes(),
        ),
    ]);

    let aad = container_aad(&cfg.study_id, &rendered.list_sha256);
    let container = encrypt_container(&archive, passphrase, &aad)?;

    let package_dir = out_dir.join(&rendered.dir_name);
    if package_dir.exists() {
        return Err(PackageError::PackageDirExists { path: package_dir });
    }
    fs::create_dir_all(&package_dir)?;

    let contents: [(&str, &[u8]); 4] = [
        (
            "generation-report.html",
            rendered.generation_report.as_bytes(),
        ),
        (
            "manifest.blinded.json",
            rendered.manifests.blinded.as_bytes(),
        ),
        ("qc.R", rendered.qc_r.as_bytes()),
        ("restricted.age", container.as_slice()),
    ];
    debug_assert_eq!(contents.len(), ENCRYPTED_PACKAGE_FILES.len());

    for &(name, bytes) in &contents {
        write_file(&package_dir.join(name), bytes)?;
    }

    let checksums = render_checksums_txt(&contents);
    write_file(&package_dir.join("checksums.txt"), checksums.as_bytes())?;

    Ok(package_dir)
}

/// Decrypt `restricted.age` **in place**, writing the 5 restricted files
/// directly into `package_dir` as plaintext.
///
/// Decrypting in place (rather than to a separate output directory) means
/// `qc.R` — which reads `list.csv` / `stream.csv` / `manifest.unblinded.json`
/// from its own directory — and the existing `verify_package` / unblinded
/// report flows work unchanged afterward.
///
/// # Errors
///
/// Returns [`PackageError::EmptyPassphrase`] for an empty passphrase,
/// [`PackageError::RestrictedFileExists`] if any target file already exists
/// (refuses to overwrite), [`PackageError::ContainerCorrupt`] if
/// `restricted.age` does not match its `checksums.txt` entry or is
/// structurally invalid, and [`PackageError::DecryptionFailed`] if AEAD
/// authentication fails (wrong passphrase, or a container that does not
/// belong to this package). Never reveals which of those two applies.
pub fn decrypt_package(package_dir: &Path, passphrase: &str) -> Result<(), PackageError> {
    if passphrase.trim().is_empty() {
        return Err(PackageError::EmptyPassphrase);
    }

    for name in RESTRICTED_FILES {
        let target = package_dir.join(name);
        if target.exists() {
            return Err(PackageError::RestrictedFileExists { path: target });
        }
    }

    let container = fs::read(package_dir.join("restricted.age"))?;
    verify_container_checksum(package_dir, &container)?;

    let (study_id, list_sha256) = read_aad_fields(package_dir)?;
    let aad = container_aad(&study_id, &list_sha256);

    let archive = decrypt_container(&container, passphrase, &aad)?;
    let files = parse_archive(&archive)?;

    for (name, bytes) in &files {
        write_file(&package_dir.join(name), bytes)?;
    }

    Ok(())
}

fn verify_container_checksum(package_dir: &Path, container: &[u8]) -> Result<(), PackageError> {
    let checksums_text = fs::read_to_string(package_dir.join("checksums.txt"))?;
    let entries = parse_checksums_txt(&checksums_text)?;
    let expected = entries
        .get("restricted.age")
        .ok_or(PackageError::ContainerCorrupt)?;
    if sha256_hex(container) != *expected {
        return Err(PackageError::ContainerCorrupt);
    }
    Ok(())
}

/// Read `study_id` and `list_sha256` from the plaintext `manifest.blinded.json`
/// to independently reconstruct the AEAD associated data used at encryption
/// time — both fields are already public (the manifest is unrestricted).
fn read_aad_fields(package_dir: &Path) -> Result<(String, String), PackageError> {
    let text = fs::read_to_string(package_dir.join("manifest.blinded.json"))?;
    let value: serde_json::Value = serde_json::from_str(text.trim_end_matches('\n'))
        .map_err(|_| PackageError::BlindedManifestJsonParse)?;
    let study_id = value
        .get("study_id")
        .and_then(serde_json::Value::as_str)
        .ok_or(PackageError::BlindedManifestJsonParse)?
        .to_owned();
    let list_sha256 = value
        .get("list_sha256")
        .and_then(serde_json::Value::as_str)
        .ok_or(PackageError::BlindedManifestJsonParse)?
        .to_owned();
    Ok((study_id, list_sha256))
}

fn container_aad(study_id: &str, list_sha256: &str) -> Vec<u8> {
    format!("clinrand-restricted-v1:{study_id}:{list_sha256}").into_bytes()
}

/// Build the plaintext multi-file archive wrapped by the AEAD, sorted by name.
fn build_archive(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut sorted: Vec<&(&str, &[u8])> = files.iter().collect();
    sorted.sort_by_key(|(name, _)| *name);

    let mut out = Vec::new();
    for (name, content) in sorted {
        let name_bytes = name.as_bytes();
        out.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        out.extend_from_slice(name_bytes);
        out.extend_from_slice(&(content.len() as u64).to_le_bytes());
        out.extend_from_slice(content);
    }
    out
}

/// Parse the plaintext multi-file archive back into `(name, content)` pairs.
fn parse_archive(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, PackageError> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    while pos < bytes.len() {
        let name_len = usize::from(read_u16_le(bytes, pos)?);
        pos += 2;
        let name_bytes = read_slice(bytes, pos, name_len)?;
        let name = std::str::from_utf8(name_bytes)
            .map_err(|_| PackageError::ContainerCorrupt)?
            .to_owned();
        pos += name_len;

        let content_len = usize_from_u64(read_u64_le(bytes, pos)?)?;
        pos += 8;
        let content = read_slice(bytes, pos, content_len)?.to_vec();
        pos += content_len;

        out.push((name, content));
    }
    Ok(out)
}

fn usize_from_u64(value: u64) -> Result<usize, PackageError> {
    usize::try_from(value).map_err(|_| PackageError::ContainerCorrupt)
}

fn read_slice(bytes: &[u8], pos: usize, len: usize) -> Result<&[u8], PackageError> {
    bytes
        .get(pos..pos.checked_add(len).ok_or(PackageError::ContainerCorrupt)?)
        .ok_or(PackageError::ContainerCorrupt)
}

fn read_u16_le(bytes: &[u8], pos: usize) -> Result<u16, PackageError> {
    let slice = read_slice(bytes, pos, 2)?;
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

fn read_u32_le(bytes: &[u8], pos: usize) -> Result<u32, PackageError> {
    let slice = read_slice(bytes, pos, 4)?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn read_u64_le(bytes: &[u8], pos: usize) -> Result<u64, PackageError> {
    let slice = read_slice(bytes, pos, 8)?;
    let mut buf = [0u8; 8];
    buf.copy_from_slice(slice);
    Ok(u64::from_le_bytes(buf))
}

fn derive_key(
    passphrase: &str,
    salt: &[u8],
    m_cost: u32,
    t_cost: u32,
    p_cost: u32,
) -> Result<Zeroizing<[u8; KEY_LEN]>, PackageError> {
    let params = Params::new(m_cost, t_cost, p_cost, Some(KEY_LEN))
        .map_err(|_| PackageError::KeyDerivationFailed)?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = Zeroizing::new([0u8; KEY_LEN]);
    argon2
        .hash_password_into(passphrase.as_bytes(), salt, key.as_mut())
        .map_err(|_| PackageError::KeyDerivationFailed)?;
    Ok(key)
}

fn encrypt_container(
    plaintext: &[u8],
    passphrase: &str,
    aad: &[u8],
) -> Result<Vec<u8>, PackageError> {
    let mut salt = [0u8; SALT_LEN];
    getrandom::fill(&mut salt).map_err(|_| PackageError::KeyDerivationFailed)?;
    let mut nonce_bytes = [0u8; NONCE_LEN];
    getrandom::fill(&mut nonce_bytes).map_err(|_| PackageError::KeyDerivationFailed)?;

    let key = derive_key(
        passphrase,
        &salt,
        ARGON2_M_COST,
        ARGON2_T_COST,
        ARGON2_P_COST,
    )?;
    let cipher = XChaCha20Poly1305::new(&ChachaKey::from(*key));
    let nonce = XNonce::from(nonce_bytes);

    let ciphertext = cipher
        .encrypt(
            &nonce,
            Payload {
                msg: plaintext,
                aad,
            },
        )
        .map_err(|_| PackageError::KeyDerivationFailed)?;

    let mut out = Vec::with_capacity(HEADER_LEN + ciphertext.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&ARGON2_M_COST.to_le_bytes());
    out.extend_from_slice(&ARGON2_T_COST.to_le_bytes());
    out.extend_from_slice(&ARGON2_P_COST.to_le_bytes());
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

fn decrypt_container(
    container: &[u8],
    passphrase: &str,
    aad: &[u8],
) -> Result<Vec<u8>, PackageError> {
    if container.len() < HEADER_LEN {
        return Err(PackageError::ContainerCorrupt);
    }
    if &container[0..4] != MAGIC {
        return Err(PackageError::ContainerCorrupt);
    }

    let m_cost = read_u32_le(container, 4)?;
    let t_cost = read_u32_le(container, 8)?;
    let p_cost = read_u32_le(container, 12)?;
    let salt = read_slice(container, 16, SALT_LEN)?;
    let nonce_bytes = read_slice(container, 16 + SALT_LEN, NONCE_LEN)?;
    let ciphertext = &container[HEADER_LEN..];

    let key = derive_key(passphrase, salt, m_cost, t_cost, p_cost)?;
    let cipher = XChaCha20Poly1305::new(&ChachaKey::from(*key));

    let mut nonce_arr = [0u8; NONCE_LEN];
    nonce_arr.copy_from_slice(nonce_bytes);
    let nonce = XNonce::from(nonce_arr);

    cipher
        .decrypt(
            &nonce,
            Payload {
                msg: ciphertext,
                aad,
            },
        )
        .map_err(|_| PackageError::DecryptionFailed)
}
