/** Types mirroring the Tauri `preview_structure` command result. */

/** One `factor = level` pair within a stratum combination. */
export interface StratumLevel {
  factor: string;
  level: string;
}

/** A single stratum combination in canonical (config) order. */
export interface StratumCombination {
  levels: StratumLevel[];
}

/** Expected per-arm total derived from ratios (integer division). */
export interface ArmTotal {
  code: string;
  label: string;
  ratio: number;
  total: number;
}

/** Blinded block structure. Never contains arm assignments. */
export type BlockPreview =
  | { kind: "none" }
  | {
      kind: "fixed";
      block_size: number;
      full_blocks_per_stratum: number;
      final_block_size: number;
      blocks_per_stratum: number;
    }
  | {
      kind: "variable";
      allowed_sizes: number[];
      note: string;
    };

/** Blinded, config-derived structure preview. No allocations, no RNG. */
export interface StructurePreview {
  study_id: string;
  method: string;
  n_strata: number;
  strata_combinations: StratumCombination[];
  list_length_per_stratum: number;
  total_records: number;
  ratio_sum: number;
  arms: ArmTotal[];
  ratio_remainder: number;
  block: BlockPreview;
  warnings: string[];
}
