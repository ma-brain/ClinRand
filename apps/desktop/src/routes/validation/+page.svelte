<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type {
    ValidationFormat,
    ValidationReportOutcome,
    ValidationTier,
  } from "$lib/types/validation";

  interface TierInfo {
    value: ValidationTier;
    label: string;
    proves: string;
    caveat: string;
  }

  // Evidential status for each tier. These mirror the wording spirit of the
  // CLI report and validation/README.md (AGENTS.md §7). They must never be
  // conflated: regression is consistency only, never correctness.
  const TIERS: TierInfo[] = [
    {
      value: "reference",
      label: "Reference",
      proves:
        "Correctness against external normative sources (RFC 8439 vectors, hand-worked derivations).",
      caveat:
        "Expected values come from an independent authority, not from this engine's own output.",
    },
    {
      value: "properties",
      label: "Properties",
      proves:
        "Invariants (P01–P09) hold for arbitrary valid configurations.",
      caveat:
        "Invariant evidence only — a passing sweep does not prove the algorithm matches an external reference. The full 1000-case sweep runs in CI.",
    },
    {
      value: "regression",
      label: "Regression",
      proves:
        "Byte-identical engine output since the last approved ALGO_VERSION.",
      caveat:
        "Consistency only — this proves nothing changed, never that the algorithm is correct. Regression fixtures arrive in Phase 9.",
    },
  ];

  let tier = $state<ValidationTier>("all");
  let format = $state<ValidationFormat>("md");

  let running = $state(false);
  let outcome = $state<ValidationReportOutcome | null>(null);
  let error = $state<string | null>(null);

  // Tiers whose evidential status to display: the selected tier, or all three
  // when "all" is chosen.
  const shownTiers = $derived(
    tier === "all" ? TIERS : TIERS.filter((t) => t.value === tier),
  );

  async function runReport(): Promise<void> {
    running = true;
    error = null;
    outcome = null;
    try {
      outcome = await invoke<ValidationReportOutcome>("run_validation_report", {
        tier,
        format,
      });
    } catch (err) {
      outcome = null;
      error =
        err instanceof Error
          ? err.message
          : "The validation report could not be produced.";
    } finally {
      running = false;
    }
  }

  function outcomeLabel(value: string): string {
    switch (value) {
      case "pass":
        return "PASS";
      case "fail":
        return "FAIL";
      case "skip":
        return "SKIP";
      default:
        return value.toUpperCase();
    }
  }
</script>

<h1>Validation</h1>

<p class="intro">
  Run the engine's validation tiers in-process and read the rendered report.
  This screen calls the Rust <code>run_validation_report</code> command; it does
  not reimplement any checks and never involves a seed.
</p>

<section class="card" aria-labelledby="evidence-heading">
  <h2 id="evidence-heading">What each tier proves</h2>
  <p class="section-note">
    The three tiers carry different evidential weight and must not be conflated.
  </p>
  <dl class="tier-list">
    {#each shownTiers as info (info.value)}
      <div class="tier-item">
        <dt>{info.label}</dt>
        <dd>
          <p class="tier-proves">{info.proves}</p>
          <p class="tier-caveat">{info.caveat}</p>
        </dd>
      </div>
    {/each}
  </dl>
</section>

<section class="card" aria-labelledby="run-heading">
  <h2 id="run-heading">Run report</h2>
  <div class="controls">
    <label>
      <span class="control-label">Tier</span>
      <select bind:value={tier} disabled={running}>
        <option value="all">All tiers</option>
        <option value="reference">Reference</option>
        <option value="properties">Properties</option>
        <option value="regression">Regression</option>
      </select>
    </label>
    <label>
      <span class="control-label">Format</span>
      <select bind:value={format} disabled={running}>
        <option value="md">Markdown</option>
        <option value="html">HTML</option>
      </select>
    </label>
    <button type="button" class="primary" onclick={runReport} disabled={running}>
      {running ? "Running…" : "Run validation report"}
    </button>
  </div>

  {#if error}
    <div class="panel error" role="alert">
      <p class="panel-title">Validation report failed</p>
      <p class="panel-body">{error}</p>
    </div>
  {/if}
</section>

{#if outcome}
  <section class="card" aria-labelledby="outcome-heading">
    <div class="outcome-header">
      <h2 id="outcome-heading">Outcome</h2>
      <span
        class="status"
        class:status-pass={outcome.outcome === "pass"}
        class:status-fail={outcome.outcome === "fail"}
        class:status-skip={outcome.outcome === "skip"}
        role="status"
      >
        {outcomeLabel(outcome.outcome)}
      </span>
    </div>
    <p class="section-note">
      {#if outcome.outcome === "fail"}
        One or more tiers failed. Treat this as a defect: do not regenerate
        fixtures or adjust expected values to make it pass.
      {:else if outcome.outcome === "skip"}
        No tier failed; at least one selected tier was skipped (e.g. regression
        fixtures are not present until Phase 9).
      {:else}
        All selected tiers passed. Remember the evidential limits above —
        regression and properties are not correctness proofs.
      {/if}
    </p>
  </section>

  <section class="card" aria-labelledby="report-heading">
    <h2 id="report-heading">Rendered report ({outcome.format})</h2>
    {#if outcome.format === "html"}
      <!-- Restrictive sandbox: no scripts, no same-origin, no plugins. The
           report is static HTML produced by the Rust command, so it renders
           fully while remaining CSP-safe and inert. -->
      <iframe
        class="report-frame"
        title="Validation report"
        sandbox=""
        srcdoc={outcome.report}
      ></iframe>
    {:else}
      <pre class="report-text">{outcome.report}</pre>
    {/if}
  </section>
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

  .card {
    padding: 1.25rem;
    margin-bottom: 1.25rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background-color: var(--surface);
  }

  .section-note {
    margin: 0 0 0.75rem;
    color: var(--text-muted);
    font-size: 0.875rem;
  }

  code {
    font-family: ui-monospace, "Cascadia Code", "Source Code Pro", Menlo, monospace;
    font-size: 0.875em;
  }

  .tier-list {
    margin: 0;
    padding: 0;
  }

  .tier-item {
    display: grid;
    grid-template-columns: 8rem 1fr;
    gap: 0.5rem 1rem;
    padding: 0.6rem 0;
  }

  .tier-item:not(:last-child) {
    border-bottom: 1px solid var(--border);
  }

  .tier-item dt {
    margin: 0;
    font-weight: 600;
    font-size: 0.9375rem;
  }

  .tier-item dd {
    margin: 0;
  }

  .tier-proves {
    margin: 0 0 0.25rem;
    font-size: 0.9375rem;
  }

  .tier-caveat {
    margin: 0;
    color: var(--text-muted);
    font-size: 0.875rem;
  }

  .controls {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: 0.75rem;
  }

  .controls label {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .control-label {
    font-size: 0.8125rem;
    color: var(--text-muted);
  }

  select {
    padding: 0.4rem 0.6rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background-color: var(--surface);
    color: var(--text);
    font: inherit;
    font-size: 0.9375rem;
  }

  button {
    font: inherit;
    cursor: pointer;
    border-radius: 4px;
  }

  button:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  button.primary {
    padding: 0.5rem 1rem;
    border: 1px solid var(--accent);
    background-color: var(--accent);
    color: #fff;
    font-size: 0.9375rem;
    font-weight: 500;
  }

  button.primary:hover:not(:disabled) {
    background-color: var(--accent-hover);
    border-color: var(--accent-hover);
  }

  .outcome-header {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
  }

  .status {
    font-size: 0.875rem;
    font-weight: 600;
    padding: 0.25rem 0.625rem;
    border: 1px solid var(--border);
    border-radius: 999px;
    color: var(--text-muted);
    white-space: nowrap;
  }

  .status-pass {
    color: #166534;
    border-color: #86efac;
    background-color: #f0fdf4;
  }

  .status-fail {
    color: var(--error-text);
    border-color: var(--error-border);
    background-color: var(--error-bg);
  }

  .status-skip {
    color: #92400e;
    border-color: #fcd34d;
    background-color: #fffbeb;
  }

  .report-text {
    margin: 0;
    padding: 1rem;
    max-height: 32rem;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: 4px;
    background-color: var(--surface);
    font-family: ui-monospace, "Cascadia Code", "Source Code Pro", Menlo, monospace;
    font-size: 0.8125rem;
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .report-frame {
    width: 100%;
    min-height: 28rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background-color: #fff;
  }

  .panel {
    padding: 1rem 1.25rem;
    margin-top: 0.75rem;
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
    white-space: pre-wrap;
    word-break: break-word;
  }

  @media (prefers-color-scheme: dark) {
    .status-pass {
      color: #86efac;
      border-color: #166534;
      background-color: #052e16;
    }

    .status-skip {
      color: #fcd34d;
      border-color: #92400e;
      background-color: #451a03;
    }
  }
</style>
