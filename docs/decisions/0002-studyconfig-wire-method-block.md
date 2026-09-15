# 0002 — StudyConfig method/block wire mapping

**Date:** 2026-09-15

## Decision

JSON `method` is a snake_case string sibling of optional `block`, not an
internally tagged enum. `simple` with a sibling `block` is a deserialize
error. `permuted_block` and `stratified_block` without `block` are
deserialize errors. Mapping is a private wire struct in `clinrand-core`;
`serde_json` stays a dev-dependency.

`numbering` is required on the wire. No `Default` / `#[serde(default)]`
was added, because the plan does not specify default `start` or `width`.
`Global` is the first `NumberingScheme` variant so any later defaulting
cannot land on `PerStratumRange`.

## Alternatives

- `#[serde(flatten)]` internally tagged `Method`. Serializes correctly,
  but `simple` plus `block` silently drops `block`.
- Default numbering to `Global { start: 1, width: 1 }` (or the §5.1
  example values) when the field is omitted.

## Reason

The brief requires `simple` to have no `block`. Ignoring a present
`block` would accept configs that do not mean what they say. Inventing
`start`/`width` would be an undocumented numbering contract.
