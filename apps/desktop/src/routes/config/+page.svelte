<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import {
    config,
    fromWire,
    loadConfig,
    toWireJson,
    type MethodName,
    type BlockKind,
    type NumberingKind,
  } from "$lib/stores/config";
  import { examples } from "$lib/examples";
  import type { ConfigIssue, ValidationOutcome } from "$lib/types/config";

  let allowLargeStrata = $state(false);
  let outcome = $state<ValidationOutcome | null>(null);
  let validating = $state(false);
  let validateError = $state<string | null>(null);

  // Map Rust error codes onto UI sections; unmapped codes fall through to the
  // general list. Codes come from apps/desktop/src-tauri/src/commands/config.rs.
  const SECTION_BY_CODE: Record<string, string> = {
    too_few_arms: "arms",
    duplicate_arm_code: "arms",
    invalid_arm_code: "arms",
    zero_ratio: "arms",
    ratio_sum_overflow: "arms",
    block_size_zero: "block",
    fixed_block_size_not_multiple: "block",
    variable_block_size_not_multiple: "block",
    empty_block_sizes: "block",
    duplicate_block_size: "block",
    block_size_too_large: "block",
    duplicate_factor_name: "strata",
    duplicate_level: "strata",
    empty_factor_levels: "strata",
    invalid_factor_name: "strata",
    invalid_level_name: "strata",
    too_many_strata: "strata",
    stratum_combination_overflow: "strata",
    stratified_block_empty_strata: "strata",
    permuted_block_non_empty_strata: "strata",
    per_stratum_range_too_small: "numbering",
  };

  const errors = $derived<ConfigIssue[]>(outcome?.errors ?? []);
  const warnings = $derived<string[]>(outcome?.warnings ?? []);

  function errorsFor(section: string): ConfigIssue[] {
    return errors.filter((e) => (SECTION_BY_CODE[e.code] ?? "general") === section);
  }

  const generalErrors = $derived(errors.filter((e) => !(e.code in SECTION_BY_CODE)));

  // The disclosure warning is shown whenever per_stratum_range is selected so
  // it is always visible (plan §5.6), even while other errors suppress the
  // backend warnings list. Prefer the backend text when present.
  const disclosureText = $derived(
    warnings.find((w) => w.includes("discloses stratum membership")) ??
      "per_stratum_range numbering discloses stratum membership in the randomization number.",
  );
  const listLengthWarning = $derived(
    warnings.find((w) => w.includes("is not a multiple of ratio sum")) ?? null,
  );
  const otherWarnings = $derived(
    warnings.filter(
      (w) =>
        !w.includes("discloses stratum membership") &&
        !w.includes("is not a multiple of ratio sum"),
    ),
  );

  function intFromEvent(e: Event): number {
    const value = (e.currentTarget as HTMLInputElement).valueAsNumber;
    return Number.isNaN(value) ? 0 : Math.trunc(value);
  }

  // --- Arms -----------------------------------------------------------------
  function addArm(): void {
    $config.arms = [...$config.arms, { code: "", label: "", ratio: 1 }];
  }
  function removeArm(index: number): void {
    $config.arms = $config.arms.filter((_, i) => i !== index);
  }

  // --- Variable block sizes -------------------------------------------------
  function addBlockSize(): void {
    $config.block_sizes = [...$config.block_sizes, 0];
  }
  function removeBlockSize(index: number): void {
    $config.block_sizes = $config.block_sizes.filter((_, i) => i !== index);
  }

  // --- Strata ---------------------------------------------------------------
  function addFactor(): void {
    $config.strata = [...$config.strata, { name: "", levels: [""] }];
  }
  function removeFactor(index: number): void {
    $config.strata = $config.strata.filter((_, i) => i !== index);
  }
  function addLevel(factorIndex: number): void {
    $config.strata[factorIndex].levels = [
      ...$config.strata[factorIndex].levels,
      "",
    ];
  }
  function removeLevel(factorIndex: number, levelIndex: number): void {
    $config.strata[factorIndex].levels = $config.strata[
      factorIndex
    ].levels.filter((_, i) => i !== levelIndex);
  }

  function loadExample(json: string): void {
    try {
      loadConfig(fromWire(JSON.parse(json)));
    } catch {
      // Bundled examples are known-good; ignore parse failures defensively.
    }
  }

  async function runValidation(json: string, allow: boolean): Promise<void> {
    validating = true;
    validateError = null;
    try {
      outcome = await invoke<ValidationOutcome>("validate_config_json", {
        json,
        allowLargeStrata: allow,
      });
    } catch (err) {
      outcome = null;
      validateError =
        err instanceof Error
          ? err.message
          : "The validation command could not be reached.";
    } finally {
      validating = false;
    }
  }

  // Debounced live validation: recompute wire JSON on any change, wait ~300ms,
  // then call the Rust command. Cleanup cancels a pending call on re-run.
  $effect(() => {
    const json = toWireJson($config);
    const allow = allowLargeStrata;
    const timer = setTimeout(() => {
      void runValidation(json, allow);
    }, 300);
    return () => clearTimeout(timer);
  });
</script>

<h1>Config builder</h1>

<div class="toolbar">
  <div class="examples">
    <span class="examples-label">Load example:</span>
    {#each examples as ex (ex.id)}
      <button type="button" class="ghost" onclick={() => loadExample(ex.json)}>
        {ex.label}
      </button>
    {/each}
  </div>

  <div
    class="status"
    class:status-valid={outcome?.ok}
    class:status-invalid={outcome && !outcome.ok}
    role="status"
    aria-live="polite"
  >
    {#if validateError}
      Validation unavailable
    {:else if validating && !outcome}
      Validating…
    {:else if outcome?.ok}
      Valid{warnings.length ? ` · ${warnings.length} warning(s)` : ""}
    {:else if outcome}
      {errors.length} error(s)
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

<!-- Study identity -->
<section class="card" aria-labelledby="study-heading">
  <h2 id="study-heading">Study</h2>
  <div class="grid">
    <label>
      <span>Schema version</span>
      <input type="text" bind:value={$config.schema_version} />
    </label>
    <label>
      <span>Study ID</span>
      <input type="text" bind:value={$config.study_id} />
    </label>
    <label>
      <span>Protocol version</span>
      <input type="text" bind:value={$config.protocol_version} />
    </label>
    <label>
      <span>List length per stratum</span>
      <input
        type="number"
        min="0"
        step="1"
        value={$config.list_length_per_stratum}
        oninput={(e) => ($config.list_length_per_stratum = intFromEvent(e))}
      />
    </label>
  </div>
  {#if listLengthWarning}
    <p class="inline-warning">{listLengthWarning}</p>
  {/if}
</section>

<!-- Arms -->
<section class="card" aria-labelledby="arms-heading">
  <h2 id="arms-heading">Arms</h2>
  <div class="rows">
    {#each $config.arms as _arm, i (i)}
      <div class="row arm-row">
        <label class="grow">
          <span>Code</span>
          <input type="text" bind:value={$config.arms[i].code} />
        </label>
        <label class="grow2">
          <span>Label</span>
          <input type="text" bind:value={$config.arms[i].label} />
        </label>
        <label class="narrow">
          <span>Ratio</span>
          <input
            type="number"
            min="0"
            step="1"
            value={$config.arms[i].ratio}
            oninput={(e) => ($config.arms[i].ratio = intFromEvent(e))}
          />
        </label>
        <button
          type="button"
          class="remove"
          onclick={() => removeArm(i)}
          disabled={$config.arms.length <= 2}
          aria-label="Remove arm"
        >
          Remove
        </button>
      </div>
    {/each}
  </div>
  <button type="button" class="ghost" onclick={addArm}>Add arm</button>
  {#each errorsFor("arms") as err (err.code + err.message)}
    <p class="inline-error">{err.message}</p>
  {/each}
</section>

<!-- Method + block -->
<section class="card" aria-labelledby="method-heading">
  <h2 id="method-heading">Method</h2>
  <div class="grid">
    <label>
      <span>Randomization method</span>
      <select
        value={$config.method}
        onchange={(e) =>
          ($config.method = (e.currentTarget as HTMLSelectElement)
            .value as MethodName)}
      >
        <option value="simple">simple</option>
        <option value="permuted_block">permuted_block</option>
        <option value="stratified_block">stratified_block</option>
      </select>
    </label>

    {#if $config.method !== "simple"}
      <label>
        <span>Block sizing</span>
        <select
          value={$config.block_kind}
          onchange={(e) =>
            ($config.block_kind = (e.currentTarget as HTMLSelectElement)
              .value as BlockKind)}
        >
          <option value="fixed">fixed</option>
          <option value="variable">variable</option>
        </select>
      </label>
    {/if}
  </div>

  {#if $config.method !== "simple"}
    {#if $config.block_kind === "fixed"}
      <div class="grid">
        <label>
          <span>Block size</span>
          <input
            type="number"
            min="0"
            step="1"
            value={$config.block_size}
            oninput={(e) => ($config.block_size = intFromEvent(e))}
          />
        </label>
      </div>
    {:else}
      <div class="rows">
        {#each $config.block_sizes as _size, i (i)}
          <div class="row">
            <label class="narrow">
              <span>Size</span>
              <input
                type="number"
                min="0"
                step="1"
                value={$config.block_sizes[i]}
                oninput={(e) => ($config.block_sizes[i] = intFromEvent(e))}
              />
            </label>
            <button
              type="button"
              class="remove"
              onclick={() => removeBlockSize(i)}
              disabled={$config.block_sizes.length <= 1}
              aria-label="Remove block size"
            >
              Remove
            </button>
          </div>
        {/each}
      </div>
      <button type="button" class="ghost" onclick={addBlockSize}>
        Add block size
      </button>
    {/if}
  {/if}

  {#each errorsFor("block") as err (err.code + err.message)}
    <p class="inline-error">{err.message}</p>
  {/each}
</section>

<!-- Strata (stratified_block only) -->
{#if $config.method === "stratified_block"}
  <section class="card" aria-labelledby="strata-heading">
    <h2 id="strata-heading">Strata</h2>
    <label class="checkbox">
      <input type="checkbox" bind:checked={allowLargeStrata} />
      <span>Allow large strata (&gt; 200 combinations)</span>
    </label>

    {#each $config.strata as _factor, fi (fi)}
      <div class="factor">
        <div class="row">
          <label class="grow">
            <span>Factor name</span>
            <input type="text" bind:value={$config.strata[fi].name} />
          </label>
          <button
            type="button"
            class="remove"
            onclick={() => removeFactor(fi)}
            aria-label="Remove factor"
          >
            Remove factor
          </button>
        </div>
        <div class="levels">
          {#each $config.strata[fi].levels as _level, li (li)}
            <div class="row">
              <label class="grow">
                <span>Level</span>
                <input
                  type="text"
                  bind:value={$config.strata[fi].levels[li]}
                />
              </label>
              <button
                type="button"
                class="remove"
                onclick={() => removeLevel(fi, li)}
                disabled={$config.strata[fi].levels.length <= 1}
                aria-label="Remove level"
              >
                Remove
              </button>
            </div>
          {/each}
          <button type="button" class="ghost" onclick={() => addLevel(fi)}>
            Add level
          </button>
        </div>
      </div>
    {/each}
    <button type="button" class="ghost" onclick={addFactor}>Add factor</button>

    {#each errorsFor("strata") as err (err.code + err.message)}
      <p class="inline-error">{err.message}</p>
    {/each}
  </section>
{:else}
  <!-- Errors that concern strata can still surface for non-stratified methods -->
  {#if errorsFor("strata").length}
    <section class="card">
      {#each errorsFor("strata") as err (err.code + err.message)}
        <p class="inline-error">{err.message}</p>
      {/each}
    </section>
  {/if}
{/if}

<!-- Numbering -->
<section class="card" aria-labelledby="numbering-heading">
  <h2 id="numbering-heading">Numbering</h2>
  <div class="grid">
    <label>
      <span>Scheme</span>
      <select
        value={$config.numbering_kind}
        onchange={(e) =>
          ($config.numbering_kind = (e.currentTarget as HTMLSelectElement)
            .value as NumberingKind)}
      >
        <option value="global">global</option>
        <option value="per_stratum_range">per_stratum_range</option>
      </select>
    </label>
    <label>
      <span>Start</span>
      <input
        type="number"
        min="0"
        step="1"
        value={$config.numbering_start}
        oninput={(e) => ($config.numbering_start = intFromEvent(e))}
      />
    </label>
    <label>
      <span>Width</span>
      <input
        type="number"
        min="0"
        step="1"
        value={$config.numbering_width}
        oninput={(e) => ($config.numbering_width = intFromEvent(e))}
      />
    </label>
    {#if $config.numbering_kind === "per_stratum_range"}
      <label>
        <span>Range size (block_size)</span>
        <input
          type="number"
          min="0"
          step="1"
          value={$config.numbering_block_size}
          oninput={(e) => ($config.numbering_block_size = intFromEvent(e))}
        />
      </label>
    {/if}
  </div>

  {#if $config.numbering_kind === "per_stratum_range"}
    <div class="panel disclosure" role="note">
      <p class="panel-title">Disclosure</p>
      <p class="panel-body">{disclosureText}</p>
    </div>
  {/if}

  {#each errorsFor("numbering") as err (err.code + err.message)}
    <p class="inline-error">{err.message}</p>
  {/each}
</section>

<!-- General errors + other warnings -->
{#if generalErrors.length}
  <section class="card">
    <h2>Other errors</h2>
    {#each generalErrors as err (err.code + err.message)}
      <p class="inline-error">{err.message}</p>
    {/each}
  </section>
{/if}

{#if otherWarnings.length}
  <section class="card">
    <h2>Warnings</h2>
    {#each otherWarnings as warn (warn)}
      <p class="inline-warning">{warn}</p>
    {/each}
  </section>
{/if}

<details class="json-view">
  <summary>Working config JSON</summary>
  <pre>{toWireJson($config)}</pre>
</details>

<style>
  h1 {
    margin: 0 0 1.25rem;
    font-size: 1.5rem;
    font-weight: 600;
  }

  h2 {
    margin: 0 0 0.75rem;
    font-size: 1rem;
    font-weight: 600;
  }

  .toolbar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    margin-bottom: 1.25rem;
  }

  .examples {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem;
  }

  .examples-label {
    color: var(--text-muted);
    font-size: 0.875rem;
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

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(12rem, 1fr));
    gap: 0.75rem 1rem;
  }

  .rows {
    display: flex;
    flex-direction: column;
    gap: 0.625rem;
    margin-bottom: 0.75rem;
  }

  .row {
    display: flex;
    align-items: flex-end;
    gap: 0.75rem;
  }

  .grow {
    flex: 1 1 8rem;
  }

  .grow2 {
    flex: 2 1 12rem;
  }

  .narrow {
    flex: 0 0 6rem;
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
    margin-bottom: 0.75rem;
    color: var(--text);
  }

  input[type="text"],
  input[type="number"],
  select {
    padding: 0.4rem 0.55rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background-color: var(--surface);
    color: var(--text);
    font: inherit;
    font-size: 0.9375rem;
  }

  input:focus,
  select:focus {
    outline: none;
    border-color: var(--accent);
  }

  .factor {
    padding: 0.875rem;
    margin-bottom: 0.875rem;
    border: 1px solid var(--border);
    border-radius: 5px;
    background-color: var(--surface-muted);
  }

  .levels {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin-top: 0.625rem;
    padding-left: 0.875rem;
    border-left: 2px solid var(--border);
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

  button.remove {
    padding: 0.4rem 0.6rem;
    border: 1px solid var(--border);
    background-color: var(--surface);
    color: var(--text-muted);
    font-size: 0.8125rem;
  }

  button.remove:hover:not(:disabled) {
    border-color: var(--error-border);
    color: var(--error-text);
  }

  .inline-error {
    margin: 0.625rem 0 0;
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--error-border);
    border-radius: 4px;
    background-color: var(--error-bg);
    color: var(--error-text);
    font-size: 0.875rem;
  }

  .inline-warning {
    margin: 0.625rem 0 0;
    padding: 0.5rem 0.75rem;
    border: 1px solid #f5d78e;
    border-radius: 4px;
    background-color: #fffbeb;
    color: #92400e;
    font-size: 0.875rem;
  }

  .panel {
    margin-top: 0.875rem;
    padding: 0.75rem 0.875rem;
    border-radius: 5px;
  }

  .panel.error {
    border: 1px solid var(--error-border);
    background-color: var(--error-bg);
    color: var(--error-text);
  }

  .panel.disclosure {
    border: 1px solid #f5d78e;
    background-color: #fffbeb;
    color: #92400e;
  }

  .panel-title {
    margin: 0 0 0.25rem;
    font-weight: 600;
    font-size: 0.875rem;
  }

  .panel-body {
    margin: 0;
    font-size: 0.875rem;
  }

  .json-view {
    margin-top: 0.5rem;
  }

  .json-view summary {
    cursor: pointer;
    color: var(--text-muted);
    font-size: 0.875rem;
  }

  .json-view pre {
    margin: 0.625rem 0 0;
    padding: 0.875rem;
    border: 1px solid var(--border);
    border-radius: 5px;
    background-color: var(--surface-muted);
    color: var(--text);
    font-family: ui-monospace, "Cascadia Code", "Source Code Pro", Menlo,
      monospace;
    font-size: 0.8125rem;
    overflow-x: auto;
  }

  @media (prefers-color-scheme: dark) {
    .status-valid {
      color: #86efac;
      border-color: #166534;
      background-color: #052e16;
    }

    .inline-warning,
    .panel.disclosure {
      border-color: #78591c;
      background-color: #292014;
      color: #fcd34d;
    }
  }
</style>
