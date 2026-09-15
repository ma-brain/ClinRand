//! Shared stdout/stderr writers for command output.

use std::io::{self, Write};

/// Write `text` to stdout.
pub fn write_stdout(text: &str) -> io::Result<()> {
    io::stdout().write_all(text.as_bytes())
}

/// Write `text` to stderr.
pub fn write_stderr(text: &str) -> io::Result<()> {
    io::stderr().write_all(text.as_bytes())
}
