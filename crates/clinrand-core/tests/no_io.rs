//! Mechanical crate-boundary check: the engine must not touch I/O or OS entropy.
//!
//! Scans `src/` rather than relying on code review to keep `clinrand-core`
//! free of `std::fs`, `std::env`, `std::time`, `getrandom`, and non-contract
//! RNG APIs (`thread_rng`, `StdRng`, `SmallRng`, `OsRng`, `rand::random`).

use std::fs;
use std::path::{Path, PathBuf};

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read src directory") {
        let entry = entry.expect("read directory entry");
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

#[test]
fn source_tree_forbids_io_and_os_entropy() {
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rust_files(&src_dir, &mut files);
    assert!(!files.is_empty(), "expected Rust sources under src/");

    let forbidden = [
        "std::fs",
        "std::env",
        "std::time",
        "getrandom",
        "thread_rng",
        "StdRng",
        "SmallRng",
        "OsRng",
        "rand::random",
    ];
    for path in files {
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        for needle in forbidden {
            assert!(
                !text.contains(needle),
                "{} contains forbidden `{needle}`",
                path.display()
            );
        }
    }
}
