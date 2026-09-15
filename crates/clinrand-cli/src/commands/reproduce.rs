//! `reproduce` subcommand — regenerate a list from an unblinded manifest.

use std::path::Path;

use chrono::Utc;
use clinrand_core::{generate_with_options, GenerationError, ValidateOptions, ALGO_VERSION};
use clinrand_package::{
    parse_unblinded_manifest, render_list_csv, sha256_hex, write_package, write_package_encrypted,
    PackageError, PackageMeta,
};

use crate::exit::ExitCode;
use crate::output::{write_stderr, write_stdout};
use crate::passphrase::prompt_new_passphrase;

/// Reproduce a randomization package from `manifest.unblinded.json` into `out_dir`.
pub fn run(json: bool, manifest_path: &str, out_dir: &str, encrypt: bool) -> ExitCode {
    let manifest_text = match std::fs::read_to_string(manifest_path) {
        Ok(text) => text,
        Err(err) => {
            let message = format!("failed to read {manifest_path}: {err}\n");
            let _ = write_stderr(&message);
            return ExitCode::IoError;
        }
    };

    let manifest = match parse_unblinded_manifest(&manifest_text) {
        Ok(manifest) => manifest,
        Err(err) => {
            let message = format!("{err}\n");
            let _ = write_stderr(&message);
            return ExitCode::IoError;
        }
    };

    if manifest.algo_version != ALGO_VERSION {
        let message = format!(
            "manifest algo_version {} does not match binary algo_version {ALGO_VERSION}\n",
            manifest.algo_version,
        );
        let _ = write_stderr(&message);
        return ExitCode::AlgoVersionMismatch;
    }

    let seed = match decode_seed_hex(&manifest.seed_hex) {
        Ok(seed) => seed,
        Err(()) => {
            let _ = write_stderr("invalid seed encoding in manifest\n");
            return ExitCode::IoError;
        }
    };

    if let Err(code) = ensure_out_dir(out_dir) {
        return code;
    }

    let passphrase = if encrypt {
        match prompt_new_passphrase() {
            Ok(passphrase) => Some(passphrase),
            Err(code) => return code,
        }
    } else {
        None
    };

    let options = ValidateOptions {
        allow_large_strata: true,
    };
    let list = match generate_with_options(&manifest.config, seed, &options) {
        Ok(list) => list,
        Err(GenerationError::InvalidConfig(_)) => {
            let _ = write_stderr("generation failed: invalid config\n");
            return ExitCode::CheckFailure;
        }
        Err(err) => {
            let message = format!("generation failed: {err}\n");
            let _ = write_stderr(&message);
            return ExitCode::CheckFailure;
        }
    };

    let list_sha256 = match render_list_csv(&manifest.config, &list) {
        Ok(csv) => sha256_hex(csv.as_bytes()),
        Err(err) => {
            let message = format!("{err}\n");
            let _ = write_stderr(&message);
            return ExitCode::CheckFailure;
        }
    };

    if list_sha256 != manifest.list_sha256 {
        let message = format!(
            "list_sha256 mismatch: manifest expects {}, regenerated list has {list_sha256}\n",
            manifest.list_sha256,
        );
        let _ = write_stderr(&message);
        return ExitCode::CheckFailure;
    }

    let generated_at = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let meta = match PackageMeta::new(&manifest.operator, &generated_at) {
        Ok(meta) => meta,
        Err(err) => return emit_package_failure(&err),
    };

    let record_count = u64::try_from(list.records.len()).unwrap_or(u64::MAX);
    let package_dir = match &passphrase {
        Some(passphrase) => write_package_encrypted(
            Path::new(out_dir),
            &manifest.config,
            &list,
            &seed,
            &meta,
            passphrase,
        ),
        None => write_package(Path::new(out_dir), &manifest.config, &list, &seed, &meta),
    };
    let package_dir = match package_dir {
        Ok(path) => path,
        Err(err) => return emit_package_failure(&err),
    };

    if let Err(code) = emit_success(json, &package_dir, &list_sha256, record_count) {
        return code;
    }

    ExitCode::Success
}

fn decode_seed_hex(hex: &str) -> Result<[u8; 32], ()> {
    if hex.len() != 64 {
        return Err(());
    }
    if !hex
        .chars()
        .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    {
        return Err(());
    }
    let mut seed = [0u8; 32];
    for (i, byte) in hex.as_bytes().chunks(2).enumerate() {
        let pair = std::str::from_utf8(byte).map_err(|_| ())?;
        seed[i] = u8::from_str_radix(pair, 16).map_err(|_| ())?;
    }
    Ok(seed)
}

fn ensure_out_dir(out_dir: &str) -> Result<(), ExitCode> {
    let path = Path::new(out_dir);
    if path.is_dir() {
        return Ok(());
    }
    if path.exists() {
        let message = format!("output path is not a directory: {out_dir}\n");
        let _ = write_stderr(&message);
        return Err(ExitCode::IoError);
    }
    let Some(parent) = path.parent() else {
        let message = format!("output directory parent does not exist: {out_dir}\n");
        let _ = write_stderr(&message);
        return Err(ExitCode::IoError);
    };
    if parent.as_os_str().is_empty() {
        return Ok(());
    }
    if parent.is_dir() {
        let message = format!("output directory does not exist: {out_dir}\n");
        let _ = write_stderr(&message);
        return Err(ExitCode::IoError);
    }
    let message = format!("output directory parent does not exist: {out_dir}\n");
    let _ = write_stderr(&message);
    Err(ExitCode::IoError)
}

fn emit_package_failure(err: &PackageError) -> ExitCode {
    let message = format!("{err}\n");
    let _ = write_stderr(&message);
    match err {
        PackageError::Io(_)
        | PackageError::PackageDirExists { .. }
        | PackageError::RestrictedFileExists { .. } => ExitCode::IoError,
        PackageError::EmptyPassphrase
        | PackageError::KeyDerivationFailed
        | PackageError::DecryptionFailed
        | PackageError::ContainerCorrupt => ExitCode::PassphraseFailure,
        _ => ExitCode::CheckFailure,
    }
}

fn emit_success(
    json: bool,
    package_dir: &Path,
    list_sha256: &str,
    record_count: u64,
) -> Result<(), ExitCode> {
    if json {
        let payload = serde_json::json!({
            "package_dir": package_dir.display().to_string(),
            "list_sha256": list_sha256,
            "record_count": record_count,
        });
        if write_stdout(&format!("{payload}\n")).is_err() {
            return Err(ExitCode::IoError);
        }
    } else {
        if write_stdout(&format!("{}\n", package_dir.display())).is_err() {
            return Err(ExitCode::IoError);
        }
        if write_stdout(&format!("{list_sha256}\n")).is_err() {
            return Err(ExitCode::IoError);
        }
    }
    Ok(())
}
