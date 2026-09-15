/** Types mirroring the Tauri `run_validation_report` command result. */

/** Validation tier selector accepted by `run_validation_report`. */
export type ValidationTier = "all" | "reference" | "properties" | "regression";

/** Rendered report format accepted by `run_validation_report`. */
export type ValidationFormat = "md" | "html";

/**
 * Structured result of running the in-process validation-tier checks.
 *
 * The report text is produced entirely in Rust (the tiers reuse the engine's
 * own reference vectors, property sweep, and regression scan). No seed is ever
 * involved. The three tiers carry different evidential weight and must not be
 * conflated (AGENTS.md §7): reference proves correctness against an external
 * source, properties proves invariants only, regression proves consistency
 * only.
 */
export interface ValidationReportOutcome {
  /** True when no tier failed. */
  ok: boolean;
  /** `"pass"`, `"fail"`, or `"skip"` for the whole report. */
  outcome: string;
  /** Rendered report text in the requested format. */
  report: string;
  /** Echoed report format (`"md"` or `"html"`). */
  format: string;
}
