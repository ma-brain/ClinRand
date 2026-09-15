<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { config, toWireJson } from "$lib/stores/config";
  import type {
    BlockPreview,
    StratumCombination,
    StructurePreview,
  } from "$lib/types/preview";

  let preview = $state<StructurePreview | null>(null);
  let previewError = $state<string | null>(null);
  let commandError = $state<string | null>(null);
  let loading = $state(false);

  function formatStratum(combo: StratumCombination): string {
    return combo.levels.map((l) => `${l.factor}=${l.level}`).join(", ");
  }

  function blockSummary(block: BlockPreview): string {
    switch (block.kind) {
      case "none":
        return "No blocking (simple randomization)";
      case "fixed":
        return `Fixed blocks of ${block.block_size}`;
      case "variable":
        return `Variable blocks (${block.allowed_sizes.join(", ")})`;
    }
  }

  let generation = 0;

  async function runPreview(json: string, gen: number): Promise<void> {
    try {
      const result = await invoke<StructurePreview>("preview_structure", { json });
      if (gen !== generation) return;
      preview = result;
      previewError = null;
      commandError = null;
    } catch (err) {
      if (gen !== generation) return;
      preview = null;
      previewError =
        err instanceof Error ? err.message : "Structure preview could not be computed.";
      commandError = null;
    } finally {
      if (gen === generation) loading = false;
    }
  }

  // Debounced reactive refresh when the shared config store changes.
  $effect(() => {
    const json = toWireJson($config);
    const gen = ++generation;
    loading = true;
    previewError = null;
    commandError = null;
    const timer = setTimeout(() => {
      void runPreview(json, gen);
    }, 300);
    return () => clearTimeout(timer);
  });
</script>

<h1>Structure preview (blinded)</h1>

<p class="intro">
  Config-derived structure only. No randomization numbers, no arm assignments, and
  no seed are shown on this screen.
</p>

<div class="toolbar">
  <div
    class="status"
    class:status-ready={!loading && preview && !previewError}
    class:status-error={!loading && previewError}
    role="status"
    aria-live="polite"
  >
    {#if commandError}
      Preview unavailable
    {:else if loading}
      Updating…
    {:else if previewError}
      Preview error
    {:else if preview}
      Ready
    {:else}
      —
    {/if}
  </div>
</div>

{#if commandError}
  <div class="panel error" role="alert">
    <p class="panel-title">Preview command error</p>
    <p class="panel-body">{commandError}</p>
  </div>
{:else if previewError}
  <div class="panel error" role="alert">
    <p class="panel-title">Could not preview structure</p>
    <p class="panel-body">{previewError}</p>
    <p class="panel-hint">
      Fix the config on the Config builder screen, or load a valid example.
    </p>
  </div>
{:else if preview}
  <section class="card" aria-labelledby="summary-heading">
    <h2 id="summary-heading">Summary</h2>
    <dl class="summary-list">
      <div>
        <dt>Study ID</dt>
        <dd>{preview.study_id}</dd>
      </div>
      <div>
        <dt>Method</dt>
        <dd><code>{preview.method}</code></dd>
      </div>
      <div>
        <dt>Stratum combinations</dt>
        <dd>{preview.n_strata}</dd>
      </div>
      <div>
        <dt>List length per stratum</dt>
        <dd>{preview.list_length_per_stratum}</dd>
      </div>
      <div>
        <dt>Total records</dt>
        <dd>{preview.total_records}</dd>
      </div>
    </dl>
  </section>

  {#if preview.strata_combinations.length > 0}
    <section class="card" aria-labelledby="strata-heading">
      <h2 id="strata-heading">Stratum combinations</h2>
      <p class="section-note">Canonical order (last factor varies fastest).</p>
      <ol class="strata-list">
        {#each preview.strata_combinations as combo, i (i)}
          <li><code>{formatStratum(combo)}</code></li>
        {/each}
      </ol>
    </section>
  {/if}

  <section class="card" aria-labelledby="arms-heading">
    <h2 id="arms-heading">Per-arm ratio totals</h2>
    <p class="section-note">
      Expected totals from integer ratio division over {preview.total_records} records
      (ratio sum {preview.ratio_sum}). These are not generated allocations.
    </p>
    <table class="data-table">
      <thead>
        <tr>
          <th scope="col">Code</th>
          <th scope="col">Label</th>
          <th scope="col">Ratio</th>
          <th scope="col">Total</th>
        </tr>
      </thead>
      <tbody>
        {#each preview.arms as arm (arm.code)}
          <tr>
            <td><code>{arm.code}</code></td>
            <td>{arm.label}</td>
            <td>{arm.ratio}</td>
            <td>{arm.total}</td>
          </tr>
        {/each}
      </tbody>
    </table>
    {#if preview.ratio_remainder > 0}
      <p class="inline-warning">
        Ratio remainder: {preview.ratio_remainder} record(s) cannot be assigned evenly
        across arms by integer division.
      </p>
    {/if}
  </section>

  <section class="card" aria-labelledby="block-heading">
    <h2 id="block-heading">Block structure</h2>
    <p class="section-note">{blockSummary(preview.block)}</p>

    {#if preview.block.kind === "fixed"}
      <dl class="summary-list compact">
        <div>
          <dt>Block size</dt>
          <dd>{preview.block.block_size}</dd>
        </div>
        <div>
          <dt>Full blocks per stratum</dt>
          <dd>{preview.block.full_blocks_per_stratum}</dd>
        </div>
        <div>
          <dt>Final block size</dt>
          <dd>{preview.block.final_block_size}</dd>
        </div>
        <div>
          <dt>Blocks per stratum</dt>
          <dd>{preview.block.blocks_per_stratum}</dd>
        </div>
      </dl>
    {:else if preview.block.kind === "variable"}
      <dl class="summary-list compact">
        <div>
          <dt>Allowed sizes</dt>
          <dd>{preview.block.allowed_sizes.join(", ")}</dd>
        </div>
      </dl>
      <p class="block-note">{preview.block.note}</p>
    {/if}
  </section>

  {#if preview.warnings.length}
    <section class="card" aria-labelledby="warnings-heading">
      <h2 id="warnings-heading">Warnings</h2>
      {#each preview.warnings as warn (warn)}
        <p class="inline-warning">{warn}</p>
      {/each}
    </section>
  {/if}
{/if}

<style>
  h1 {
    margin: 0 0 0.75rem;
    font-size: 1.5rem;
    font-weight: 600;
  }

  h2 {
    margin: 0 0 0.75rem;
    font-size: 1rem;
    font-weight: 600;
  }

  .intro {
    margin: 0 0 1.25rem;
    color: var(--text-muted);
    font-size: 0.9375rem;
  }

  .toolbar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: flex-end;
    gap: 0.75rem;
    margin-bottom: 1.25rem;
  }

  .status {
    font-size: 0.875rem;
    font-weight: 500;
    padding: 0.25rem 0.625rem;
    border: 1px solid var(--border);
    border-radius: 999px;
    color: var(--text-muted);
    white-space: nowrap;
  }

  .status-ready {
    color: #166534;
    border-color: #86efac;
    background-color: #f0fdf4;
  }

  .status-error {
    color: var(--error-text);
    border-color: var(--error-border);
    background-color: var(--error-bg);
  }

  .card {
    padding: 1.25rem;
    margin-bottom: 1.25rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background-color: var(--surface);
  }

  .section-note {
    margin: -0.35rem 0 0.75rem;
    color: var(--text-muted);
    font-size: 0.875rem;
  }

  .summary-list {
    margin: 0;
    padding: 0;
  }

  .summary-list div {
    display: grid;
    grid-template-columns: 11rem 1fr;
    gap: 0.5rem 1rem;
    padding: 0.35rem 0;
  }

  .summary-list div:not(:last-child) {
    border-bottom: 1px solid var(--border);
    padding-bottom: 0.5rem;
    margin-bottom: 0.25rem;
  }

  .summary-list.compact div:not(:last-child) {
    border-bottom: none;
    padding-bottom: 0.35rem;
    margin-bottom: 0;
  }

  .summary-list dt {
    margin: 0;
    color: var(--text-muted);
    font-weight: 400;
    font-size: 0.875rem;
  }

  .summary-list dd {
    margin: 0;
    font-size: 0.9375rem;
  }

  code {
    font-family: ui-monospace, "Cascadia Code", "Source Code Pro", Menlo, monospace;
    font-size: 0.875em;
  }

  .strata-list {
    margin: 0;
    padding-left: 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .strata-list li {
    font-size: 0.9375rem;
  }

  .data-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.9375rem;
  }

  .data-table th,
  .data-table td {
    padding: 0.45rem 0.65rem;
    text-align: left;
    border-bottom: 1px solid var(--border);
  }

  .data-table th {
    color: var(--text-muted);
    font-weight: 500;
    font-size: 0.8125rem;
  }

  .data-table tbody tr:last-child td {
    border-bottom: none;
  }

  .block-note {
    margin: 0.75rem 0 0;
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background-color: var(--surface-muted);
    color: var(--text-muted);
    font-size: 0.875rem;
  }

  .inline-warning {
    margin: 0.75rem 0 0;
    padding: 0.5rem 0.75rem;
    border: 1px solid #f5d78e;
    border-radius: 4px;
    background-color: #fffbeb;
    color: #92400e;
    font-size: 0.875rem;
  }

  .card > .inline-warning:not(:first-of-type) {
    margin-top: 0.5rem;
  }

  .panel {
    padding: 1rem 1.25rem;
    border-radius: 6px;
  }

  .panel.error {
    border: 1px solid var(--error-border);
    background-color: var(--error-bg);
    color: var(--error-text);
  }

  .panel-title {
    margin: 0 0 0.25rem;
    font-weight: 600;
    font-size: 0.9375rem;
  }

  .panel-body {
    margin: 0;
    font-size: 0.9375rem;
  }

  .panel-hint {
    margin: 0.5rem 0 0;
    font-size: 0.875rem;
    opacity: 0.9;
  }

  @media (prefers-color-scheme: dark) {
    .status-ready {
      color: #86efac;
      border-color: #166534;
      background-color: #052e16;
    }

    .inline-warning {
      border-color: #78591c;
      background-color: #292014;
      color: #fcd34d;
    }
  }
</style>
