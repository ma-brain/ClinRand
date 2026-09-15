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
    type WorkingConfig,
  } from "$lib/stores/config";
  import { examples } from "$lib/examples";
  import type { ConfigIssue, ValidationOutcome } from "$lib/types/config";

  let allowLargeStrata = $state(false);
  let outcome = $state<ValidationOutcome | null>(null);
  let validating = $state(false);
  let validateError = $state<string | null>(null);

  const errors = $derived<ConfigIssue[]>(outcome?.errors ?? []);
  const warnings = $derived<string[]>(outcome?.warnings ?? []);

  /**
   * Errors resolved to specific controls. Value-bearing backend codes are
   * matched against the current store rows (via the value carried in the
   * message); anything that cannot be pinned to a control falls back to a
   * section summary or the general list. Codes come from
   * apps/desktop/src-tauri/src/commands/config.rs.
   */
  interface MappedErrors {
    arm: Map<number, ConfigIssue[]>;
    blockSize: Map<number, ConfigIssue[]>;
    factor: Map<number, ConfigIssue[]>;
    level: Map<string, ConfigIssue[]>;
    fixedBlock: ConfigIssue[];
    numberingBlockSize: ConfigIssue[];
    sectionArms: ConfigIssue[];
    sectionBlock: ConfigIssue[];
    sectionStrata: ConfigIssue[];
    general: ConfigIssue[];
  }

  function pushMap<K>(map: Map<K, ConfigIssue[]>, key: K, issue: ConfigIssue): void {
    const existing = map.get(key);
    if (existing) {
      existing.push(issue);
    } else {
      map.set(key, [issue]);
    }
  }

  function mapErrors(issues: ConfigIssue[], cfg: WorkingConfig): MappedErrors {
    const arm = new Map<number, ConfigIssue[]>();
    const blockSize = new Map<number, ConfigIssue[]>();
    const factor = new Map<number, ConfigIssue[]>();
    const level = new Map<string, ConfigIssue[]>();
    const fixedBlock: ConfigIssue[] = [];
    const numberingBlockSize: ConfigIssue[] = [];
    const sectionArms: ConfigIssue[] = [];
    const sectionBlock: ConfigIssue[] = [];
    const sectionStrata: ConfigIssue[] = [];
    const general: ConfigIssue[] = [];

    const armsByCode = (code: string): number[] =>
      cfg.arms.flatMap((a, i) => (a.code === code ? [i] : []));
    const blockSizesEqual = (n: number): number[] =>
      cfg.block_sizes.flatMap((s, i) => (s === n ? [i] : []));
    const factorsByName = (name: string): number[] =>
      cfg.strata.flatMap((f, i) => (f.name === name ? [i] : []));

    const attachArm = (issue: ConfigIssue, code: string | undefined): void => {
      const idx = code !== undefined ? armsByCode(code) : [];
      if (idx.length) idx.forEach((i) => pushMap(arm, i, issue));
      else sectionArms.push(issue);
    };
    const attachSize = (issue: ConfigIssue, token: string | undefined): void => {
      const idx = token !== undefined ? blockSizesEqual(Number(token)) : [];
      if (idx.length) idx.forEach((i) => pushMap(blockSize, i, issue));
      else sectionBlock.push(issue);
    };
    const attachFactor = (issue: ConfigIssue, name: string | undefined): void => {
      const idx = name !== undefined ? factorsByName(name) : [];
      if (idx.length) idx.forEach((i) => pushMap(factor, i, issue));
      else sectionStrata.push(issue);
    };
    const attachLevel = (
      issue: ConfigIssue,
      match: RegExpMatchArray | null,
    ): void => {
      if (!match) {
        sectionStrata.push(issue);
        return;
      }
      const lvl = match[1];
      const fname = match[2];
      let matched = false;
      cfg.strata.forEach((f, fi) => {
        if (f.name !== fname) return;
        f.levels.forEach((l, li) => {
          if (l === lvl) {
            pushMap(level, `${fi}:${li}`, issue);
            matched = true;
          }
        });
      });
      if (!matched) sectionStrata.push(issue);
    };

    for (const issue of issues) {
      const m = issue.message;
      switch (issue.code) {
        case "too_few_arms":
        case "ratio_sum_overflow":
          sectionArms.push(issue);
          break;
        case "duplicate_arm_code":
          attachArm(issue, m.match(/^duplicate arm code (.*)$/)?.[1]);
          break;
        case "invalid_arm_code":
          attachArm(issue, m.match(/^arm code (.*) is invalid$/)?.[1]);
          break;
        case "zero_ratio":
          attachArm(issue, m.match(/^arm (.*) has ratio 0$/)?.[1]);
          break;
        case "block_size_zero":
        case "empty_block_sizes":
          sectionBlock.push(issue);
          break;
        case "fixed_block_size_not_multiple":
          fixedBlock.push(issue);
          break;
        case "variable_block_size_not_multiple":
          attachSize(issue, m.match(/^variable block size (\d+) /)?.[1]);
          break;
        case "duplicate_block_size":
          attachSize(issue, m.match(/^duplicate variable block size (\d+)$/)?.[1]);
          break;
        case "block_size_too_large":
          attachSize(issue, m.match(/^variable block size (\d+) exceeds 24$/)?.[1]);
          break;
        case "duplicate_factor_name":
          attachFactor(issue, m.match(/^duplicate stratum factor name (.*)$/)?.[1]);
          break;
        case "empty_factor_levels":
          attachFactor(issue, m.match(/^factor (.*) has no levels$/)?.[1]);
          break;
        case "invalid_factor_name":
          attachFactor(issue, m.match(/^factor name (.*) is invalid$/)?.[1]);
          break;
        case "duplicate_level":
          attachLevel(issue, m.match(/^duplicate level (.+?) in factor (.+)$/));
          break;
        case "invalid_level_name":
          attachLevel(issue, m.match(/^level (.+?) of factor (.+)$/));
          break;
        case "too_many_strata":
        case "stratum_combination_overflow":
        case "stratified_block_empty_strata":
        case "permuted_block_non_empty_strata":
          sectionStrata.push(issue);
          break;
        case "per_stratum_range_too_small":
          numberingBlockSize.push(issue);
          break;
        default:
          general.push(issue);
      }
    }

    return {
      arm,
      blockSize,
      factor,
      level,
      fixedBlock,
      numberingBlockSize,
      sectionArms,
      sectionBlock,
      sectionStrata,
      general,
    };
  }

  const mapped = $derived(mapErrors(errors, $config));

  const armErr = (i: number): ConfigIssue[] => mapped.arm.get(i) ?? [];
  const blockSizeErr = (i: number): ConfigIssue[] => mapped.blockSize.get(i) ?? [];
  const factorErr = (i: number): ConfigIssue[] => mapped.factor.get(i) ?? [];
  const levelErr = (fi: number, li: number): ConfigIssue[] =>
    mapped.level.get(`${fi}:${li}`) ?? [];

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

  // Monotonic generation guard: only the newest request may touch shared state,
  // so a slow older request can never overwrite a newer result or flip the
  // `validating` flag off while a newer request is still in flight.
  let generation = 0;

  async function runValidation(
    json: string,
    allow: boolean,
    gen: number,
  ): Promise<void> {
    try {
      const result = await invoke<ValidationOutcome>("validate_config_json", {
        json,
        allowLargeStrata: allow,
      });
      if (gen !== generation) return;
      outcome = result;
      validateError = null;
    } catch (err) {
      if (gen !== generation) return;
      outcome = null;
      validateError =
        err instanceof Error
          ? err.message
          : "The validation command could not be reached.";
    } finally {
      if (gen === generation) validating = false;
    }
  }

  // Debounced live validation. Every edit bumps the generation and immediately
  // marks the result as checking (so the status never shows a stale verdict),
  // then waits ~300ms before calling the Rust command.
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
    class:status-valid={!validating && outcome?.ok}
    class:status-invalid={!validating && outcome && !outcome.ok}
    role="status"
    aria-live="polite"
  >
    {#if validateError}
      Validation unavailable
    {:else if validating}
      Checking…
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
      <div class="field-group">
        <div class="row arm-row">
          <label class="grow">
            <span>Code</span>
            <input
              type="text"
              bind:value={$config.arms[i].code}
              aria-invalid={armErr(i).length > 0}
              aria-describedby={armErr(i).length ? `arm-err-${i}` : undefined}
            />
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
              aria-invalid={armErr(i).length > 0}
              aria-describedby={armErr(i).length ? `arm-err-${i}` : undefined}
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
        {#if armErr(i).length}
          <div id={`arm-err-${i}`} class="field-errors">
            {#each armErr(i) as err (err.code + err.message)}
              <p class="inline-error">{err.message}</p>
            {/each}
          </div>
        {/if}
      </div>
    {/each}
  </div>
  <button type="button" class="ghost" onclick={addArm}>Add arm</button>
  {#each mapped.sectionArms as err (err.code + err.message)}
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
            aria-invalid={mapped.fixedBlock.length > 0}
            aria-describedby={mapped.fixedBlock.length ? "fixed-block-err" : undefined}
          />
        </label>
      </div>
      {#if mapped.fixedBlock.length}
        <div id="fixed-block-err" class="field-errors">
          {#each mapped.fixedBlock as err (err.code + err.message)}
            <p class="inline-error">{err.message}</p>
          {/each}
        </div>
      {/if}
    {:else}
      <div class="rows">
        {#each $config.block_sizes as _size, i (i)}
          <div class="field-group">
            <div class="row">
              <label class="narrow">
                <span>Size</span>
                <input
                  type="number"
                  min="0"
                  step="1"
                  value={$config.block_sizes[i]}
                  oninput={(e) => ($config.block_sizes[i] = intFromEvent(e))}
                  aria-invalid={blockSizeErr(i).length > 0}
                  aria-describedby={blockSizeErr(i).length
                    ? `block-size-err-${i}`
                    : undefined}
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
            {#if blockSizeErr(i).length}
              <div id={`block-size-err-${i}`} class="field-errors">
                {#each blockSizeErr(i) as err (err.code + err.message)}
                  <p class="inline-error">{err.message}</p>
                {/each}
              </div>
            {/if}
          </div>
        {/each}
      </div>
      <button type="button" class="ghost" onclick={addBlockSize}>
        Add block size
      </button>
    {/if}
  {/if}

  {#each mapped.sectionBlock as err (err.code + err.message)}
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
        <div class="field-group">
          <div class="row">
            <label class="grow">
              <span>Factor name</span>
              <input
                type="text"
                bind:value={$config.strata[fi].name}
                aria-invalid={factorErr(fi).length > 0}
                aria-describedby={factorErr(fi).length
                  ? `factor-err-${fi}`
                  : undefined}
              />
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
          {#if factorErr(fi).length}
            <div id={`factor-err-${fi}`} class="field-errors">
              {#each factorErr(fi) as err (err.code + err.message)}
                <p class="inline-error">{err.message}</p>
              {/each}
            </div>
          {/if}
        </div>
        <div class="levels">
          {#each $config.strata[fi].levels as _level, li (li)}
            <div class="field-group">
              <div class="row">
                <label class="grow">
                  <span>Level</span>
                  <input
                    type="text"
                    bind:value={$config.strata[fi].levels[li]}
                    aria-invalid={levelErr(fi, li).length > 0}
                    aria-describedby={levelErr(fi, li).length
                      ? `level-err-${fi}-${li}`
                      : undefined}
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
              {#if levelErr(fi, li).length}
                <div id={`level-err-${fi}-${li}`} class="field-errors">
                  {#each levelErr(fi, li) as err (err.code + err.message)}
                    <p class="inline-error">{err.message}</p>
                  {/each}
                </div>
              {/if}
            </div>
          {/each}
          <button type="button" class="ghost" onclick={() => addLevel(fi)}>
            Add level
          </button>
        </div>
      </div>
    {/each}
    <button type="button" class="ghost" onclick={addFactor}>Add factor</button>

    {#each mapped.sectionStrata as err (err.code + err.message)}
      <p class="inline-error">{err.message}</p>
    {/each}
  </section>
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
          aria-invalid={mapped.numberingBlockSize.length > 0}
          aria-describedby={mapped.numberingBlockSize.length
            ? "numbering-block-size-err"
            : undefined}
        />
      </label>
    {/if}
  </div>

  {#if mapped.numberingBlockSize.length}
    <div id="numbering-block-size-err" class="field-errors">
      {#each mapped.numberingBlockSize as err (err.code + err.message)}
        <p class="inline-error">{err.message}</p>
      {/each}
    </div>
  {/if}

  {#if $config.numbering_kind === "per_stratum_range"}
    <div class="panel disclosure" role="note">
      <p class="panel-title">Disclosure</p>
      <p class="panel-body">{disclosureText}</p>
    </div>
  {/if}
</section>

<!-- General errors + other warnings -->
{#if mapped.general.length}
  <section class="card">
    <h2>Other errors</h2>
    {#each mapped.general as err (err.code + err.message)}
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

  .field-group {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
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

  input[aria-invalid="true"] {
    border-color: var(--error-border);
    background-color: var(--error-bg);
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

  .field-errors {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }

  .inline-error {
    margin: 0;
    padding: 0.4rem 0.65rem;
    border: 1px solid var(--error-border);
    border-radius: 4px;
    background-color: var(--error-bg);
    color: var(--error-text);
    font-size: 0.8125rem;
  }

  section > .inline-error {
    margin-top: 0.625rem;
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
