/**
 * Bundled example configs, copied verbatim from the repository `examples/`
 * directory. Loaded entirely from the app bundle — never fetched over the
 * network (the app has no network capability). Study IDs match
 * `^(DEMO|TEST|EXAMPLE)-`.
 */

export interface ConfigExample {
  /** Stable id used as a key. */
  id: string;
  /** Button label. */
  label: string;
  /** Raw wire JSON, parsed on load. */
  json: string;
}

export const examples: ConfigExample[] = [
  {
    id: "simple",
    label: "Simple",
    json: `{
  "schema_version": "1.0",
  "study_id": "DEMO-SIMPLE-1",
  "protocol_version": "1.0",
  "arms": [
    { "code": "A", "label": "Active", "ratio": 1 },
    { "code": "P", "label": "Placebo", "ratio": 1 }
  ],
  "method": "simple",
  "strata": [],
  "list_length_per_stratum": 24,
  "numbering": { "kind": "global", "start": 10001, "width": 5 }
}`,
  },
  {
    id: "permuted-block-fixed",
    label: "Permuted block (fixed)",
    json: `{
  "schema_version": "1.0",
  "study_id": "DEMO-PB-FIXED-1",
  "protocol_version": "1.0",
  "arms": [
    { "code": "A", "label": "Active", "ratio": 1 },
    { "code": "P", "label": "Placebo", "ratio": 1 }
  ],
  "method": "permuted_block",
  "block": { "kind": "fixed", "size": 4 },
  "strata": [],
  "list_length_per_stratum": 24,
  "numbering": { "kind": "global", "start": 10001, "width": 5 }
}`,
  },
  {
    id: "stratified-block-variable",
    label: "Stratified block (variable)",
    json: `{
  "schema_version": "1.0",
  "study_id": "DEMO-201",
  "protocol_version": "2.1",
  "arms": [
    { "code": "A", "label": "Investigational product 50 mg", "ratio": 2 },
    { "code": "P", "label": "Placebo", "ratio": 1 }
  ],
  "method": "stratified_block",
  "block": { "kind": "variable", "sizes": [6, 9] },
  "strata": [
    { "name": "site", "levels": ["001", "002", "003"] },
    { "name": "agegrp", "levels": ["LT65", "GE65"] }
  ],
  "list_length_per_stratum": 36,
  "numbering": { "kind": "global", "start": 10001, "width": 5 }
}`,
  },
  {
    id: "per-stratum-range",
    label: "Per-stratum range",
    json: `{
  "schema_version": "1.0",
  "study_id": "EXAMPLE-RANGE-1",
  "protocol_version": "1.0",
  "arms": [
    { "code": "A", "label": "Active", "ratio": 1 },
    { "code": "P", "label": "Placebo", "ratio": 1 }
  ],
  "method": "stratified_block",
  "block": { "kind": "fixed", "size": 4 },
  "strata": [
    { "name": "site", "levels": ["001", "002"] }
  ],
  "list_length_per_stratum": 24,
  "numbering": {
    "kind": "per_stratum_range",
    "start": 20001,
    "block_size": 100,
    "width": 5
  }
}`,
  },
];
