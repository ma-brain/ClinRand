/** Types mirroring the Tauri `verify_package` command result. */

/**
 * Structured verification report for a package directory.
 *
 * Produced by re-hashing the package files against `checksums.txt` and
 * re-running the property checks against the stored list. The seed is never
 * read or surfaced.
 */
export interface VerifyOutcome {
  /** True when both checksums and required properties pass. */
  ok: boolean;
  /** True when every file matches its `checksums.txt` digest. */
  checksums_ok: boolean;
  /** True when all required property checks pass, or trivially true when
   * `properties_checked` is `false`. */
  properties_ok: boolean;
  /** False when the package's restricted files are still encrypted in
   * `restricted.age` — property checks need the plaintext list and were
   * skipped. Decrypt the package first to enable them. */
  properties_checked: boolean;
  /** Human-readable checksum failures. */
  checksum_failures: string[];
  /** Human-readable required property failures (`Pxx: detail`). */
  property_failures: string[];
}
