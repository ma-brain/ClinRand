//! Shared CSV field escaping for package emitters.
//!
//! UTF-8, LF, no BOM. Quote a field only when it contains comma, `"`, or
//! newline; escape `"` as `""` (minimal RFC 4180-style).

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
