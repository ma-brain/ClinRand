//! Hidden-input passphrase prompts for `generate --encrypt` / `decrypt`.
//!
//! `rpassword` reads directly from `/dev/tty` (or the platform equivalent)
//! without echoing input, when stdin is an interactive terminal. When stdin
//! is not a terminal (piped or redirected — scripted use, or an automated
//! test), there is no terminal to hide input on regardless, so this falls
//! back to a plain line read from stdin: the same pattern `age`/`ssh-keygen`
//! use, and the only way to drive `--encrypt` / `decrypt` non-interactively.
//! The passphrase still never appears in argv or the environment either way.
//! `docs/decisions/0008-cli-decrypt-command-and-passphrase-ux.md` records why
//! `generate --encrypt` asks twice (a typo'd passphrase permanently loses the
//! restricted files) while `decrypt` asks once.

use std::io::{self, BufRead, IsTerminal};

use crate::exit::ExitCode;
use crate::output::write_stderr;

/// Prompt for a new passphrase, asking twice to catch typos.
///
/// Reports an empty entry or a mismatched confirmation to stderr and returns
/// [`ExitCode::PassphraseFailure`] — never partially proceeds with an
/// unconfirmed passphrase.
pub fn prompt_new_passphrase() -> Result<String, ExitCode> {
    let first = read_hidden("Passphrase to encrypt restricted files: ")?;
    if first.trim().is_empty() {
        let _ = write_stderr("passphrase must not be empty\n");
        return Err(ExitCode::PassphraseFailure);
    }
    let second = read_hidden("Confirm passphrase: ")?;
    if first != second {
        let _ = write_stderr("passphrases did not match\n");
        return Err(ExitCode::PassphraseFailure);
    }
    Ok(first)
}

/// Prompt once for an existing passphrase, for `decrypt`.
pub fn prompt_existing_passphrase() -> Result<String, ExitCode> {
    read_hidden("Passphrase: ")
}

fn read_hidden(prompt: &str) -> Result<String, ExitCode> {
    let result = if io::stdin().is_terminal() {
        rpassword::prompt_password(prompt)
    } else {
        read_line_non_interactive(prompt)
    };
    result.map_err(|err| {
        let _ = write_stderr(&format!("could not read passphrase: {err}\n"));
        ExitCode::IoError
    })
}

/// Read one line from stdin when it is not an interactive terminal. There is
/// no echo to suppress in that case (the input is already piped/redirected
/// by whatever is driving this process), so this is a plain, unhidden read.
fn read_line_non_interactive(prompt: &str) -> io::Result<String> {
    let _ = write_stderr(prompt);
    let mut line = String::new();
    io::stdin().lock().read_line(&mut line)?;
    Ok(line.trim_end_matches(['\n', '\r']).to_owned())
}
