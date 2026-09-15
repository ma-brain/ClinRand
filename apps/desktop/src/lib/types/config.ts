/** Types mirroring the Tauri `validate_config_json` command result. */

/** One config error rendered for the UI: a stable machine code plus message. */
export interface ConfigIssue {
  /** Stable snake_case identifier for the failure kind (e.g. `too_few_arms`). */
  code: string;
  /** Human-readable message. */
  message: string;
}

/** Structured result of validating a config JSON string. */
export interface ValidationOutcome {
  /** True when the JSON parsed and validation returned no errors. */
  ok: boolean;
  /** Every independent validation error (or a single `invalid_json` entry). */
  errors: ConfigIssue[];
  /** Non-fatal warnings (disclosure / truncation notes). */
  warnings: string[];
}
