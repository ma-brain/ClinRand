/**
 * Shared working-config store for the desktop builder.
 *
 * Holds the study config being edited in a flattened, UI-friendly shape and
 * exposes helpers that serialize it into the published wire JSON (plan §5.1)
 * consumed by the Tauri commands. Task 5 (preview) and Task 6 (generate) reuse
 * this store so the operator edits one config across screens.
 *
 * No allocation, hashing, or canonicalization happens here — only the shape of
 * the config. The seed never appears in this module.
 */

import { get, writable } from "svelte/store";

/** Randomization method name (wire `method` string). */
export type MethodName = "simple" | "permuted_block" | "stratified_block";

/** Block sizing scheme (wire `block.kind`). */
export type BlockKind = "fixed" | "variable";

/** Randomization-number scheme (wire `numbering.kind`). */
export type NumberingKind = "global" | "per_stratum_range";

/** A treatment arm as edited in the form. */
export interface ArmInput {
  code: string;
  label: string;
  ratio: number;
}

/** A stratification factor as edited in the form. */
export interface FactorInput {
  name: string;
  levels: string[];
}

/**
 * Flattened working config. Block and numbering variant fields are kept even
 * when the current method/scheme does not use them, so switching back and forth
 * does not lose the operator's input. Serialization emits only the relevant
 * fields.
 */
export interface WorkingConfig {
  schema_version: string;
  study_id: string;
  protocol_version: string;
  arms: ArmInput[];
  method: MethodName;
  block_kind: BlockKind;
  block_size: number;
  block_sizes: number[];
  strata: FactorInput[];
  list_length_per_stratum: number;
  numbering_kind: NumberingKind;
  numbering_start: number;
  numbering_width: number;
  numbering_block_size: number;
}

/**
 * A fresh, valid-by-default config. Study ID matches `^(DEMO|TEST|EXAMPLE)-`
 * per AGENTS.md §4.10.
 */
export function defaultConfig(): WorkingConfig {
  return {
    schema_version: "1.0",
    study_id: "DEMO-001",
    protocol_version: "1.0",
    arms: [
      { code: "A", label: "Active", ratio: 1 },
      { code: "P", label: "Placebo", ratio: 1 },
    ],
    method: "simple",
    block_kind: "fixed",
    block_size: 4,
    block_sizes: [4, 6],
    strata: [],
    list_length_per_stratum: 24,
    numbering_kind: "global",
    numbering_start: 10001,
    numbering_width: 5,
    numbering_block_size: 100,
  };
}

/** The single shared working config. */
export const config = writable<WorkingConfig>(defaultConfig());

function intOr0(value: number): number {
  return Number.isFinite(value) ? Math.trunc(value) : 0;
}

/** Build the wire-shape object (plan §5.1) from a working config. */
export function toWireConfig(c: WorkingConfig): Record<string, unknown> {
  const numbering: Record<string, unknown> =
    c.numbering_kind === "per_stratum_range"
      ? {
          kind: "per_stratum_range",
          start: intOr0(c.numbering_start),
          block_size: intOr0(c.numbering_block_size),
          width: intOr0(c.numbering_width),
        }
      : {
          kind: "global",
          start: intOr0(c.numbering_start),
          width: intOr0(c.numbering_width),
        };

  const wire: Record<string, unknown> = {
    schema_version: c.schema_version,
    study_id: c.study_id,
    protocol_version: c.protocol_version,
    arms: c.arms.map((a) => ({
      code: a.code,
      label: a.label,
      ratio: intOr0(a.ratio),
    })),
    method: c.method,
    // Only stratified_block carries strata on the wire; simple/permuted must be
    // empty (validation rejects a non-empty strata array for permuted_block).
    strata:
      c.method === "stratified_block"
        ? c.strata.map((f) => ({ name: f.name, levels: [...f.levels] }))
        : [],
    list_length_per_stratum: intOr0(c.list_length_per_stratum),
    numbering,
  };

  if (c.method !== "simple") {
    wire.block =
      c.block_kind === "variable"
        ? { kind: "variable", sizes: c.block_sizes.map(intOr0) }
        : { kind: "fixed", size: intOr0(c.block_size) };
  }

  return wire;
}

/** Serialize the working config to pretty wire JSON. */
export function toWireJson(c: WorkingConfig): string {
  return JSON.stringify(toWireConfig(c), null, 2);
}

/** Wire JSON string for the current store value (for Task 5 / Task 6). */
export function currentWireJson(): string {
  return toWireJson(get(config));
}

/** Current wire-shape object for the store value. */
export function currentWireConfig(): Record<string, unknown> {
  return toWireConfig(get(config));
}

/**
 * Best-effort conversion of a wire-shape object into a working config, used to
 * load bundled examples. Missing fields fall back to `defaultConfig()`.
 */
export function fromWire(raw: unknown): WorkingConfig {
  const base = defaultConfig();
  if (typeof raw !== "object" || raw === null) {
    return base;
  }
  const wire = raw as Record<string, unknown>;

  const method: MethodName =
    wire.method === "permuted_block" || wire.method === "stratified_block"
      ? wire.method
      : "simple";

  const block = wire.block as Record<string, unknown> | undefined;
  const blockKind: BlockKind = block?.kind === "variable" ? "variable" : "fixed";

  const numbering = wire.numbering as Record<string, unknown> | undefined;
  const numberingKind: NumberingKind =
    numbering?.kind === "per_stratum_range" ? "per_stratum_range" : "global";

  const arms = Array.isArray(wire.arms)
    ? (wire.arms as Record<string, unknown>[]).map((a) => ({
        code: String(a.code ?? ""),
        label: String(a.label ?? ""),
        ratio: Number(a.ratio) || 0,
      }))
    : base.arms;

  const strata = Array.isArray(wire.strata)
    ? (wire.strata as Record<string, unknown>[]).map((f) => ({
        name: String(f.name ?? ""),
        levels: Array.isArray(f.levels) ? (f.levels as unknown[]).map(String) : [],
      }))
    : [];

  return {
    schema_version: String(wire.schema_version ?? base.schema_version),
    study_id: String(wire.study_id ?? base.study_id),
    protocol_version: String(wire.protocol_version ?? base.protocol_version),
    arms: arms.length >= 1 ? arms : base.arms,
    method,
    block_kind: blockKind,
    block_size:
      block?.kind === "fixed" ? Number(block.size) || 0 : base.block_size,
    block_sizes:
      block?.kind === "variable" && Array.isArray(block.sizes)
        ? (block.sizes as unknown[]).map((s) => Number(s) || 0)
        : base.block_sizes,
    strata,
    list_length_per_stratum:
      Number(wire.list_length_per_stratum) || 0,
    numbering_kind: numberingKind,
    numbering_start: Number(numbering?.start) || 0,
    numbering_width: Number(numbering?.width) || 0,
    numbering_block_size:
      numbering?.kind === "per_stratum_range"
        ? Number(numbering.block_size) || 0
        : base.numbering_block_size,
  };
}

/** Replace the working config (e.g. when loading an example). */
export function loadConfig(next: WorkingConfig): void {
  config.set(next);
}
