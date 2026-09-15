//! `decrypt` subcommand — decrypt a package's `restricted.age` in place.

use std::path::Path;

use clinrand_package::{decrypt_package, PackageError};

use crate::exit::ExitCode;
use crate::output::{write_stderr, write_stdout};
use crate::passphrase::prompt_existing_passphrase;

/// Decrypt `restricted.age` in `package_dir`, writing the 5 restricted
/// files directly into it as plaintext.
pub fn run(json: bool, package_dir: &str) -> ExitCode {
    let passphrase = match prompt_existing_passphrase() {
        Ok(passphrase) => passphrase,
        Err(code) => return code,
    };

    if let Err(err) = decrypt_package(Path::new(package_dir), &passphrase) {
        return emit_failure(&err);
    }

    if json {
        let payload = serde_json::json!({ "package_dir": package_dir });
        if write_stdout(&format!("{payload}\n")).is_err() {
            return ExitCode::IoError;
        }
    } else if write_stdout(&format!("{package_dir}\n")).is_err() {
        return ExitCode::IoError;
    }

    ExitCode::Success
}

fn emit_failure(err: &PackageError) -> ExitCode {
    let message = format!("{err}\n");
    let _ = write_stderr(&message);
    match err {
        PackageError::Io(_) | PackageError::RestrictedFileExists { .. } => ExitCode::IoError,
        PackageError::EmptyPassphrase
        | PackageError::KeyDerivationFailed
        | PackageError::DecryptionFailed
        | PackageError::ContainerCorrupt => ExitCode::PassphraseFailure,
        _ => ExitCode::CheckFailure,
    }
}
