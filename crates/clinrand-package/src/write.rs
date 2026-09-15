//! Filesystem package writer and `checksums.txt` (plan §6).
//!
//! Writes exact bytes from the list/stream renderers,
//! [`crate::build_manifests`], HTML report renderers, and [`crate::render_qc_r`].
//! Does not re-serialize manifests or write encrypted containers.

use std::fs;
use std::path::{Path, PathBuf};

use clinrand_core::{GeneratedList, StudyConfig};

use crate::canonical::{config_sha256, sha256_hex};
use crate::error::PackageError;
use crate::list::{render_list_csv, render_list_json};
use crate::manifest::{build_manifests, seed_sha256, PackageMeta};
use crate::qc::render_qc_r;
use crate::report::{render_generation_report, render_unblinded_report, ReportFileHashes};
use crate::stream::render_stream_csv;

/// Filenames written into a package directory (excluding `checksums.txt`).
///
/// Sorted lexicographically — this is also the `checksums.txt` line order.
const PACKAGE_FILES: &[&str] = &[
    "generation-report.html",
    "list.csv",
    "list.json",
    "manifest.blinded.json",
    "manifest.unblinded.json",
    "qc.R",
    "stream.csv",
    "unblinded-report.html",
];

/// Write a ClinRand output package under `out_dir`.
///
/// Creates
/// `<out_dir>/<study_id>_<generated_at compact>_<first 8 hex of list_sha256>/`
/// containing `list.csv`, `list.json`, `stream.csv`, both manifests, both HTML
/// reports, `qc.R`, and `checksums.txt`.
///
/// Manifest file bytes are exactly those returned by [`build_manifests`].
/// `list.csv` / `list.json` / `stream.csv` are exactly the renderer outputs.
/// HTML reports are exactly the report-renderer outputs. The seed appears only
/// in `manifest.unblinded.json` on disk (never in the blinded report).
///
/// # Errors
///
/// Returns [`PackageError::PackageDirExists`] if the target package directory
/// already exists (refuse overwrite). Propagates render / manifest failures
/// and filesystem I/O errors. [`PackageError`](crate::PackageError)
/// [`Display`](std::fmt::Display) never includes the seed.
pub fn write_package(
    out_dir: &Path,
    cfg: &StudyConfig,
    list: &GeneratedList,
    seed: &[u8; 32],
    meta: &PackageMeta,
) -> Result<PathBuf, PackageError> {
    let rendered = render_package(cfg, list, seed, meta)?;

    let package_dir = out_dir.join(&rendered.dir_name);
    if package_dir.exists() {
        return Err(PackageError::PackageDirExists { path: package_dir });
    }
    fs::create_dir_all(&package_dir).map_err(PackageError::from)?;

    let contents: [(&str, &[u8]); 8] = [
        (
            "generation-report.html",
            rendered.generation_report.as_bytes(),
        ),
        ("list.csv", rendered.list_csv.as_bytes()),
        ("list.json", rendered.list_json.as_bytes()),
        (
            "manifest.blinded.json",
            rendered.manifests.blinded.as_bytes(),
        ),
        (
            "manifest.unblinded.json",
            rendered.manifests.unblinded.as_bytes(),
        ),
        ("qc.R", rendered.qc_r.as_bytes()),
        ("stream.csv", rendered.stream_csv.as_bytes()),
        (
            "unblinded-report.html",
            rendered.unblinded_report.as_bytes(),
        ),
    ];

    debug_assert_eq!(
        contents.len(),
        PACKAGE_FILES.len(),
        "checksum set must match PACKAGE_FILES"
    );
    for &(name, bytes) in &contents {
        write_file(&package_dir.join(name), bytes)?;
    }

    let checksums = render_checksums_txt(&contents);
    write_file(&package_dir.join("checksums.txt"), checksums.as_bytes())?;

    Ok(package_dir)
}

/// Every rendered byte of a package, before any file is written to disk.
///
/// Shared by [`write_package`] and `write_package_encrypted` (`crate::encrypt`)
/// so both writers derive identical content from `(cfg, list, seed, meta)` —
/// only which files land on disk as plaintext differs.
pub(crate) struct RenderedPackage {
    pub(crate) list_csv: String,
    pub(crate) list_json: String,
    pub(crate) stream_csv: String,
    pub(crate) manifests: crate::manifest::ManifestPair,
    pub(crate) generation_report: String,
    pub(crate) unblinded_report: String,
    pub(crate) qc_r: String,
    /// SHA-256 (lowercase hex) of `list.csv` bytes — also the package
    /// directory name's hash suffix.
    pub(crate) list_sha256: String,
    /// `<study_id>_<generated_at compact>_<first 8 hex of list_sha256>`.
    pub(crate) dir_name: String,
}

pub(crate) fn render_package(
    cfg: &StudyConfig,
    list: &GeneratedList,
    seed: &[u8; 32],
    meta: &PackageMeta,
) -> Result<RenderedPackage, PackageError> {
    let list_csv = render_list_csv(cfg, list)?;
    let list_json = render_list_json(cfg, list)?;
    let stream_csv = render_stream_csv(&list.stream)?;
    let manifests = build_manifests(cfg, list, seed, meta)?;

    let list_sha256 = sha256_hex(list_csv.as_bytes());
    let hashes = ReportFileHashes {
        list_sha256: list_sha256.clone(),
        list_json_sha256: sha256_hex(list_json.as_bytes()),
        stream_sha256: sha256_hex(stream_csv.as_bytes()),
        manifest_blinded_sha256: sha256_hex(manifests.blinded.as_bytes()),
        manifest_unblinded_sha256: sha256_hex(manifests.unblinded.as_bytes()),
        config_sha256: config_sha256(cfg).map_err(PackageError::from)?,
        seed_sha256: seed_sha256(seed),
    };

    let generation_report = render_generation_report(cfg, list, meta, &hashes);
    let unblinded_report = render_unblinded_report(cfg, list, meta, &hashes);
    let qc_r = render_qc_r(cfg)?;

    let dir_name = format!(
        "{}_{}_{}",
        cfg.study_id,
        compact_generated_at(&meta.generated_at),
        &list_sha256[..8]
    );

    Ok(RenderedPackage {
        list_csv,
        list_json,
        stream_csv,
        manifests,
        generation_report,
        unblinded_report,
        qc_r,
        list_sha256,
        dir_name,
    })
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
/// one line per file in `contents`, sorted by filename, each line ending in
/// `\n`. Generic over the file set — `write_package` passes all 8 package
/// files; `write_package_encrypted` (`crate::encrypt`) passes its 4.
pub(crate) fn render_checksums_txt(contents: &[(&str, &[u8])]) -> String {
    let mut pairs: Vec<(&str, &[u8])> = contents.to_vec();
    pairs.sort_by(|a, b| a.0.cmp(b.0));
    pairs
        .into_iter()
        .map(|(name, bytes)| format!("{}  {}\n", sha256_hex(bytes), name))
        .collect()
}

pub(crate) fn write_file(path: &Path, bytes: &[u8]) -> Result<(), PackageError> {
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
            ("generation-report.html", b"g\n"),
            ("qc.R", b"q\n"),
            ("unblinded-report.html", b"ub\n"),
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
