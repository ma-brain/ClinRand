//! Shared CSV field escaping and parsing for package emitters/readers.
//!
//! UTF-8, LF, no BOM. Quote a field only when it contains comma, `"`, or
//! newline; escape `"` as `""` (minimal RFC 4180-style).

use crate::error::PackageError;

/// Escape one CSV field. Synthetic fixtures normally need no quotes.
pub(crate) fn escape_csv_field(field: &str) -> String {
    if field.contains([',', '"', '\n', '\r']) {
        let mut out = String::with_capacity(field.len() + 2);
        out.push('"');
        for ch in field.chars() {
            if ch == '"' {
                out.push('"');
            }
            out.push(ch);
        }
        out.push('"');
        out
    } else {
        field.to_owned()
    }
}

/// Parse CSV text into rows of unescaped fields (minimal RFC 4180-style).
///
/// Handles quoted fields with embedded commas and newlines. A trailing `\n`
/// after the last row does not produce an extra empty row.
pub(crate) fn parse_csv_rows(text: &str) -> Result<Vec<Vec<String>>, PackageError> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' if !in_quotes => in_quotes = true,
            '"' if in_quotes => {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    in_quotes = false;
                }
            }
            ',' if !in_quotes => row.push(std::mem::take(&mut field)),
            '\n' if !in_quotes => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
            }
            '\r' if !in_quotes => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
            }
            c => field.push(c),
        }
    }

    if in_quotes {
        return Err(PackageError::ListCsvParse {
            detail: "unclosed quoted CSV field".into(),
        });
    }

    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }

    Ok(rows)
}
