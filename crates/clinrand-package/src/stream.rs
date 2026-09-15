//! stream.csv renderer (plan §6 package tree).
//!
//! Encoding: UTF-8, LF line endings, no BOM. The returned string ends with
//! exactly one trailing `\n` after the last row (or after the header when
//! the stream is empty). Columns: `index,bound,value,purpose`. Purpose is
//! snake_case (`block_size`, `permutation`, `simple_allocation`).

use clinrand_core::{DrawPurpose, StreamLog};

use crate::csv_util::escape_csv_field;
use crate::error::PackageError;

/// Render `stream.csv` bytes as a UTF-8 string.
///
/// Pure function of `stream`. Does not include the seed.
pub fn render_stream_csv(stream: &StreamLog) -> Result<String, PackageError> {
    let mut out = String::from("index,bound,value,purpose\n");
    for draw in &stream.draws {
        out.push_str(&draw.index.to_string());
        out.push(',');
        out.push_str(&draw.bound.to_string());
        out.push(',');
        out.push_str(&draw.value.to_string());
        out.push(',');
        out.push_str(&escape_csv_field(purpose_snake(draw.purpose)));
        out.push('\n');
    }
    Ok(out)
}

fn purpose_snake(purpose: DrawPurpose) -> &'static str {
    match purpose {
        DrawPurpose::BlockSize => "block_size",
        DrawPurpose::Permutation => "permutation",
        DrawPurpose::SimpleAllocation => "simple_allocation",
    }
}
