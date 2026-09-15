<script lang="ts">
  import { goto } from "$app/navigation";
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { config, currentWireJson, toWireJson } from "$lib/stores/config";
  import type { GenerateOutcome } from "$lib/types/generate";
  import type { ValidationOutcome } from "$lib/types/config";

  let operator = $state("");
  let outDir = $state<string | null>(null);
  let allowLargeStrata = $state(false);
  let showConfirm = $state(false);
  let generating = $state(false);
  let generateError = $state<string | null>(null);
  let outcome = $state<GenerateOutcome | null>(null);

  let validating = $state(false);
  let validation = $state<ValidationOutcome | null>(null);
  let validateError = $state<string | null>(null);

  const operatorTrimmed = $derived(operator.trim());
  const canSubmit = $derived(
    !generating &&
      operatorTrimmed.length > 0 &&
      outDir !== null &&
      validation?.ok === true,
  );

  let generation = 0;

  async function runValidation(json: string, allow: boolean, gen: number): Promise<void> {
    try {
      const result = await invoke<ValidationOutcome>("validate_config_json", {
        json,
        allowLargeStrata: allow,
      });
      if (gen !== generation) return;
      validation = result;
      validateError = null;
    } catch (err) {
      if (gen !== generation) return;
      validation = null;
      validateError =
        err instanceof Error
          ? err.message
          : "The validation command could not be reached.";
    } finally {
      if (gen === generation) validating = false;
    }
  }

  $effect(() => {
    const json = toWireJson($config);
    const allow = allowLargeStrata;
    const gen = ++generation;
    validating = true;
    const timer = setTimeout(() => {
      void runValidation(json, allow, gen);
    }, 300);
    return () => clearTimeout(timer);
  });

  async function pickDirectory(): Promise<void> {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Select destination folder for package",
      });
      if (typeof selected === "string") {
        outDir = selected;
      }
    } catch (err) {
      generateError =
        err instanceof Error ? err.message : "Could not open folder picker.";
    }
  }

  function requestGenerate(): void {
    generateError = null;
    if (!canSubmit || outDir === null) return;
    showConfirm = true;
  }

  function cancelConfirm(): void {
    showConfirm = false;
  }

  async function confirmGenerate(): Promise<void> {
    if (!canSubmit || outDir === null) {
      showConfirm = false;
      return;
    }

    showConfirm = false;
    generating = true;
    generateError = null;
    outcome = null;

    try {
      const result = await invoke<GenerateOutcome>("generate_package", {
        configJson: currentWireJson(),
        outDir,
        operator: operatorTrimmed,
        allowLargeStrata,
      });
      outcome = result;
    } catch (err) {
      outcome = null;
      generateError =
        err instanceof Error ? err.message : "Package generation failed.";
    } finally {
      generating = false;
    }
  }

  function openPackageViewer(): void {
    if (!outcome) return;
    try {
      sessionStorage.setItem("clinrand:lastPackageDir", outcome.package_dir);
    } catch {
      // sessionStorage may be unavailable in some embed contexts; navigation still works.
    }
    void goto("/package");
  }
</script>

<h1>Generate package</h1>

<p class="intro">
  Write a randomization package from the current config. The package contains
  unblinded material (allocation list and unblinded manifest/report). The seed is
  never shown in this application.
</p>

<div class="toolbar">
  <div
    class="status"
    class:status-valid={!validating && validation?.ok}
    class:status-invalid={!validating && validation && !validation.ok}
    role="status"
    aria-live="polite"
  >
    {#if validateError}
      Validation unavailable
    {:else if validating}
      Checking config…
    {:else if validation?.ok}
      Config valid{validation.warnings.length
        ? ` · ${validation.warnings.length} warning(s)`
        : ""}
    {:else if validation}
      Fix {validation.errors.length} config error(s) before generating
    {:else}
      —
    {/if}
  </div>
</div>

{#if validateError}
  <div class="panel error" role="alert">
    <p class="panel-title">Validation command error</p>
    <p class="panel-body">{validateError}</p>
  </div>
{/if}

<section class="card" aria-labelledby="operator-heading">
  <h2 id="operator-heading">Operator</h2>
  <label class="grow">
    <span>Operator name (required)</span>
    <input
      type="text"
      bind:value={operator}
      autocomplete="name"
      placeholder="Statistician name"
      disabled={generating}
      required
    />
  </label>
</section>

<section class="card" aria-labelledby="destination-heading">
  <h2 id="destination-heading">Destination</h2>
  <p class="section-note">
    Choose an existing folder or a new folder name under an existing parent. The
    package directory will be created inside this location.
  </p>
  <div class="row">
    <button type="button" class="ghost" onclick={pickDirectory} disabled={generating}>
      Choose folder…
    </button>
    <p class="path-display" aria-live="polite">
      {#if outDir}
        <code>{outDir}</code>
      {:else}
        <span class="path-placeholder">No folder selected</span>
      {/if}
    </p>
  </div>
</section>

{#if $config.method === "stratified_block"}
  <section class="card" aria-labelledby="options-heading">
    <h2 id="options-heading">Options</h2>
    <label class="checkbox">
      <input type="checkbox" bind:checked={allowLargeStrata} disabled={generating} />
      <span>Allow large strata (&gt; 200 combinations)</span>
    </label>
  </section>
{/if}

<div class="actions">
  <button
    type="button"
    class="primary"
    onclick={requestGenerate}
    disabled={!canSubmit}
  >
    {generating ? "Generating…" : "Generate package"}
  </button>
</div>

{#if generateError}
  <div class="panel error" role="alert">
    <p class="panel-title">Generation failed</p>
    <p class="panel-body">{generateError}</p>
  </div>
{/if}

{#if outcome}
  <section class="card success" aria-labelledby="result-heading">
    <h2 id="result-heading">Package written</h2>
    <dl class="summary-list">
      <div>
        <dt>Package directory</dt>
        <dd><code>{outcome.package_dir}</code></dd>
      </div>
      <div>
        <dt>list_sha256</dt>
        <dd><code>{outcome.list_sha256}</code></dd>
      </div>
      <div>
        <dt>Record count</dt>
        <dd>{outcome.record_count}</dd>
      </div>
    </dl>

    {#if outcome.warnings.length}
      <div class="warnings-block">
        <p class="warnings-title">Warnings</p>
        {#each outcome.warnings as warn (warn)}
          <p class="inline-warning">{warn}</p>
        {/each}
      </div>
    {/if}

    <div class="result-actions">
      <button type="button" class="ghost" onclick={openPackageViewer}>
        Open in package viewer
      </button>
    </div>
  </section>
{/if}

{#if showConfirm}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="modal-backdrop"
    role="dialog"
    aria-modal="true"
    aria-labelledby="confirm-title"
    tabindex="-1"
    onkeydown={(e) => {
      if (e.key === "Escape") cancelConfirm();
    }}
  >
    <div class="modal">
      <h2 id="confirm-title">Unblinded output confirmation</h2>
      <div class="confirm-body">
        <p class="confirm-lead">
          You are about to generate a package that contains
          <strong>unblinded material</strong>.
        </p>
        <ul>
          <li>Full randomization list with arm assignments (<code>list.csv</code>)</li>
          <li>Unblinded manifest (<code>manifest.unblinded.json</code>)</li>
          <li>Unblinded generation report</li>
        </ul>
        <p>
          Store the package securely. Do not share unblinded files with blinded
          study staff. The seed is written only inside the package and is never
          displayed here.
        </p>
      </div>
      <div class="modal-actions">
        <button type="button" class="ghost" onclick={cancelConfirm}>Cancel</button>
        <button type="button" class="danger" onclick={confirmGenerate}>
          Generate unblinded package
        </button>
      </div>
    </div>
  </div>
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

  .status-valid {
    color: #166534;
    border-color: #86efac;
    background-color: #f0fdf4;
  }

  .status-invalid {
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

  .card.success {
    border-color: #86efac;
  }

  .section-note {
    margin: -0.35rem 0 0.75rem;
    color: var(--text-muted);
    font-size: 0.875rem;
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 0.875rem;
    color: var(--text-muted);
  }

  label.checkbox {
    flex-direction: row;
    align-items: center;
    gap: 0.5rem;
    color: var(--text);
  }

  .grow {
    flex: 1 1 auto;
  }

  input[type="text"] {
    padding: 0.4rem 0.55rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background-color: var(--surface);
    color: var(--text);
    font: inherit;
    font-size: 0.9375rem;
  }

  input:focus {
    outline: none;
    border-color: var(--accent);
  }

  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.75rem;
  }

  .path-display {
    margin: 0;
    flex: 1 1 12rem;
    font-size: 0.875rem;
    word-break: break-all;
  }

  .path-placeholder {
    color: var(--text-muted);
    font-style: italic;
  }

  code {
    font-family: ui-monospace, "Cascadia Code", "Source Code Pro", Menlo, monospace;
    font-size: 0.875em;
  }

  .actions {
    margin-bottom: 1.25rem;
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

  button.ghost {
    padding: 0.35rem 0.7rem;
    border: 1px solid var(--border);
    background-color: var(--surface);
    color: var(--text);
    font-size: 0.875rem;
  }

  button.ghost:hover:not(:disabled) {
    border-color: var(--accent);
    color: var(--accent);
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

  button.danger {
    padding: 0.5rem 1rem;
    border: 1px solid #b91c1c;
    background-color: #b91c1c;
    color: #fff;
    font-size: 0.9375rem;
    font-weight: 500;
  }

  button.danger:hover:not(:disabled) {
    background-color: #991b1b;
    border-color: #991b1b;
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

  .warnings-block {
    margin-top: 0.75rem;
  }

  .warnings-title {
    margin: 0 0 0.35rem;
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--text-muted);
  }

  .inline-warning {
    margin: 0.35rem 0 0;
    padding: 0.5rem 0.75rem;
    border: 1px solid #f5d78e;
    border-radius: 4px;
    background-color: #fffbeb;
    color: #92400e;
    font-size: 0.875rem;
  }

  .result-actions {
    margin-top: 1rem;
  }

  .panel {
    padding: 1rem 1.25rem;
    margin-bottom: 1.25rem;
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

  .modal-backdrop {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1.5rem;
    background-color: rgb(0 0 0 / 0.45);
  }

  .modal {
    width: min(100%, 28rem);
    padding: 1.25rem 1.5rem;
    border: 2px solid #b91c1c;
    border-radius: 8px;
    background-color: var(--surface);
    box-shadow: 0 12px 40px rgb(0 0 0 / 0.25);
  }

  .modal h2 {
    color: #b91c1c;
    font-size: 1.125rem;
  }

  .confirm-body {
    font-size: 0.9375rem;
  }

  .confirm-lead {
    margin: 0 0 0.75rem;
  }

  .confirm-body ul {
    margin: 0 0 0.75rem;
    padding-left: 1.25rem;
  }

  .confirm-body li {
    margin-bottom: 0.35rem;
  }

  .confirm-body p {
    margin: 0;
    color: var(--text-muted);
    font-size: 0.875rem;
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.75rem;
    margin-top: 1.25rem;
  }

  @media (prefers-color-scheme: dark) {
    .status-valid {
      color: #86efac;
      border-color: #166534;
      background-color: #052e16;
    }

    .card.success {
      border-color: #166534;
    }

    .inline-warning {
      border-color: #78591c;
      background-color: #292014;
      color: #fcd34d;
    }

    .modal {
      border-color: #ef4444;
    }

    .modal h2 {
      color: #fca5a5;
    }
  }
</style>
