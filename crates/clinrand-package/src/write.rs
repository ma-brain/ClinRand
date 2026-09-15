//! Filesystem package writer and `checksums.txt` (plan §6).
//!
//! Writes exact bytes from the list/stream renderers and
//! [`crate::build_manifests`]; does not re-serialize manifests.
//! Does not emit `qc.R`, HTML reports, or encrypted containers.

use std::fs;
use std::path::{Path, PathBuf};

use clinrand_core::{GeneratedList, StudyConfig};

use crate::canonical::sha256_hex;
use crate::error::PackageError;
use crate::list::{render_list_csv, render_list_json};
use crate::manifest::{build_manifests, PackageMeta};
use crate::stream::render_stream_csv;

/// Filenames written into a package directory (excluding `checksums.txt`).
///
/// Sorted lexicographically — this is also the `checksums.txt` line order.
const PACKAGE_FILES: &[&str] = &[
    "list.csv",
    "list.json",
    "manifest.blinded.json",
    "manifest.unblinded.json",
    "stream.csv",
];

/// Write a ClinRand output package under `out_dir`.
///
/// Creates
/// `<out_dir>/<study_id>_<generated_at compact>_<first 8 hex of list_sha256>/`
/// containing `list.csv`, `list.json`, `stream.csv`, both manifests, and
/// `checksums.txt`. Does not write `qc.R` or HTML reports.
///
/// Manifest file bytes are exactly those returned by [`build_manifests`].
/// `list.csv` / `list.json` / `stream.csv` are exactly the renderer outputs.
/// The seed appears only in `manifest.unblinded.json` on disk.
///
/// # Errors
///
/// Propagates render / manifest failures and filesystem I/O errors.
/// [`PackageError`](crate::PackageError) [`Display`](std::fmt::Display)
/// never includes the seed.
pub fn write_package(
    out_dir: &Path,
    cfg: &StudyConfig,
    list: &GeneratedList,
    seed: &[u8; 32],
    meta: &PackageMeta,
) -> Result<PathBuf, PackageError> {
    let list_csv = render_list_csv(cfg, list)?;
    let list_json = render_list_json(cfg, list)?;
    let stream_csv = render_stream_csv(&list.stream)?;
    let manifests = build_manifests(cfg, list, seed, meta)?;

    let list_sha256 = sha256_hex(list_csv.as_bytes());
    let dir_name = format!(
        "{}_{}_{}",
        cfg.study_id,
        compact_generated_at(&meta.generated_at),
        &list_sha256[..8]
    );
    let package_dir = out_dir.join(dir_name);

    fs::create_dir_all(&package_dir).map_err(PackageError::from)?;

    let contents: [(&str, &[u8]); 5] = [
        ("list.csv", list_csv.as_bytes()),
        ("list.json", list_json.as_bytes()),
        ("manifest.blinded.json", manifests.blinded.as_bytes()),
        ("manifest.unblinded.json", manifests.unblinded.as_bytes()),
        ("stream.csv", stream_csv.as_bytes()),
    ];

    for &(name, bytes) in &contents {
        write_file(&package_dir.join(name), bytes)?;
    }

    let checksums = render_checksums_txt(&contents);
    write_file(&package_dir.join("checksums.txt"), checksums.as_bytes())?;

    Ok(package_dir)
}

/// Normalize `generated_at` for the package directory name.
///
/// Accepts either a full ISO-8601 UTC timestamp (`2026-09-15T14:42:10Z`)
/// or an already-compact form (`20260915T144210Z`). Strips `-`, `:`, and
/// fractional-second punctuation; keeps `T` and trailing `Z`.
pub fn compact_generated_at(generated_at: &str) -> String {
    generated_at
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

/// `checksums.txt` body: GNU `sha256sum` text mode (`hex  filename`),
/// one line per package file **except** `checksums.txt` itself, sorted by
/// filename, each line ending in `\n`.
fn render_checksums_txt(contents: &[(&str, &[u8])]) -> String {
    debug_assert_eq!(
        contents.len(),
        PACKAGE_FILES.len(),
        "checksum set must match PACKAGE_FILES"
    );
    let mut pairs: Vec<(&str, &[u8])> = contents.to_vec();
    pairs.sort_by(|a, b| a.0.cmp(b.0));
    pairs
        .into_iter()
        .map(|(name, bytes)| format!("{}  {}\n", sha256_hex(bytes), name))
        .collect()
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<(), PackageError> {
    fs::write(path, bytes).map_err(PackageError::from)
}

#[cfg(test)]
mod tests {
    use super::{compact_generated_at, render_checksums_txt, PACKAGE_FILES};

    #[test]
    fn compact_generated_at_strips_iso_punctuation() {
        assert_eq!(
            compact_generated_at("2026-09-15T14:42:10Z"),
            "20260915T144210Z"
        );
    }

    #[test]
    fn compact_generated_at_keeps_already_compact() {
        assert_eq!(compact_generated_at("20260915T144210Z"), "20260915T144210Z");
    }

    #[test]
    fn checksums_lines_are_sorted_gnu_sha256sum_text_mode() {
        let body = render_checksums_txt(&[
            ("stream.csv", b"s\n"),
            ("list.csv", b"l\n"),
            ("list.json", b"j\n"),
            ("manifest.unblinded.json", b"u\n"),
            ("manifest.blinded.json", b"b\n"),
        ]);
        let names: Vec<&str> = body
            .lines()
            .map(|line| {
                let parts: Vec<&str> = line.splitn(2, "  ").collect();
                assert_eq!(parts.len(), 2);
                assert_eq!(parts[0].len(), 64);
                parts[1]
            })
            .collect();
        assert_eq!(names, PACKAGE_FILES);
        assert!(body.ends_with('\n'));
    }
}
