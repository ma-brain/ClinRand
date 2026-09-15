/** Types mirroring the Tauri `generate_package` command result. */

/** Successful package generation. Never includes seed material. */
export interface GenerateOutcome {
  /** Absolute path of the written package directory. */
  package_dir: string;
  /** SHA-256 (lowercase hex) of the canonical `list.csv` bytes. */
  list_sha256: string;
  /** Number of allocation records in the list. */
  record_count: number;
  /** Non-fatal validation warnings surfaced to the operator. */
  warnings: string[];
}
