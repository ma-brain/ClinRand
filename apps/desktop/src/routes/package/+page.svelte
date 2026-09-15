<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import { exists, readTextFile } from "@tauri-apps/plugin-fs";
  import type { VerifyOutcome } from "$lib/types/verify";

  const REPORT_FILE = "generation-report.html";

  let lastPackageDir = $state<string | null>(null);
  let packageDir = $state<string | null>(null);
  let pickerError = $state<string | null>(null);

  let verifying = $state(false);
  let verifyOutcome = $state<VerifyOutcome | null>(null);
  let verifyError = $state<string | null>(null);

  let loadingReport = $state(false);
  let reportHtml = $state<string | null>(null);
  let reportError = $state<string | null>(null);

  // Monotonic request token. Each package load increments it; async helpers
  // only commit their results if their captured token is still current, so an
  // earlier, slower load can never overwrite a later selection's state.
  let loadToken = 0;
  const loading = $derived(verifying || loadingReport);

  onMount(() => {
    try {
      const stored = sessionStorage.getItem("clinrand:lastPackageDir");
      lastPackageDir = stored && stored.length > 0 ? stored : null;
    } catch {
      // sessionStorage may be unavailable in some embed contexts; ignore.
      lastPackageDir = null;
    }
  });

  /**
   * Join a directory and file name using the directory's own separator.
   * Tauri returns native paths; the Rust `fs` layer accepts `/` on every
   * platform, but we preserve `\` on Windows-style paths for clarity.
   */
  function joinPath(dir: string, file: string): string {
    const usesBackslash = dir.includes("\\") && !dir.includes("/");
    const separator = usesBackslash ? "\\" : "/";
    const trimmed = dir.replace(/[\\/]+$/, "");
    return `${trimmed}${separator}${file}`;
  }

  async function runVerify(dir: string, token: number): Promise<void> {
    try {
      const outcome = await invoke<VerifyOutcome>("verify_package", {
        packageDir: dir,
      });
      if (token !== loadToken) return;
      verifyOutcome = outcome;
      verifyError = null;
    } catch (err) {
      if (token !== loadToken) return;
      verifyOutcome = null;
      verifyError =
        err instanceof Error
          ? err.message
          : "The package could not be verified.";
    } finally {
      if (token === loadToken) verifying = false;
    }
  }

  async function loadReport(dir: string, token: number): Promise<void> {
    try {
      const path = joinPath(dir, REPORT_FILE);
      const present = await exists(path);
      if (token !== loadToken) return;
      if (!present) {
        reportHtml = null;
        reportError = `This folder does not contain ${REPORT_FILE}. Confirm you selected a ClinRand package directory.`;
        return;
      }
      const html = await readTextFile(path);
      if (token !== loadToken) return;
      reportHtml = html;
      reportError = null;
    } catch (err) {
      if (token !== loadToken) return;
      reportHtml = null;
      reportError =
        err instanceof Error
          ? err.message
          : "The blinded report could not be read.";
    } finally {
      if (token === loadToken) loadingReport = false;
    }
  }

  async function loadPackage(dir: string): Promise<void> {
    const token = ++loadToken;
    packageDir = dir;
    verifyOutcome = null;
    verifyError = null;
    reportHtml = null;
    reportError = null;
    verifying = true;
    loadingReport = true;
    await Promise.all([runVerify(dir, token), loadReport(dir, token)]);
  }

  async function openPackage(): Promise<void> {
    pickerError = null;
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Select ClinRand package directory",
        defaultPath: lastPackageDir ?? undefined,
      });
      if (typeof selected === "string") {
        await loadPackage(selected);
      }
    } catch (err) {
      pickerError =
        err instanceof Error
          ? err.message
          : "Could not open the folder picker.";
    }
  }
</script>

<h1>Package viewer</h1>

<p class="intro">
  Open a generated package to verify its checksums and required properties, and
  to read the blinded generation report. This screen never shows the seed and
  never opens unblinded material.
</p>

<section class="card" aria-labelledby="open-heading">
  <h2 id="open-heading">Open package</h2>
  {#if lastPackageDir}
    <p class="section-note">
      Last generated package this session:
    </p>
    <p class="path-display">
      <code>{lastPackageDir}</code>
    </p>
    <p class="section-note">
      Select it (or any package directory) in the folder picker to load it.
    </p>
  {:else}
    <p class="section-note">
      Choose a package directory produced by the generate screen.
    </p>
  {/if}
  <div class="row">
    <button type="button" class="primary" onclick={openPackage} disabled={loading}>
      {#if loading}
        Loading…
      {:else}
        {lastPackageDir ? "Open a package…" : "Choose package folder…"}
      {/if}
    </button>
    <p class="path-display" aria-live="polite">
      {#if packageDir}
        <code>{packageDir}</code>
      {:else}
        <span class="path-placeholder">No package loaded</span>
      {/if}
    </p>
  </div>
  {#if pickerError}
    <div class="panel error" role="alert">
      <p class="panel-title">Could not open package</p>
      <p class="panel-body">{pickerError}</p>
    </div>
  {/if}
</section>

{#if packageDir}
  <section class="card" aria-labelledby="verify-heading">
    <div class="verify-header">
      <h2 id="verify-heading">Verification</h2>
      {#if !verifying && verifyOutcome}
        <span
          class="status"
          class:status-valid={verifyOutcome.ok}
          class:status-invalid={!verifyOutcome.ok}
          role="status"
        >
          {verifyOutcome.ok ? "Package verified" : "Verification failed"}
        </span>
      {/if}
    </div>

    {#if verifying}
      <p class="section-note" aria-live="polite">Verifying package…</p>
    {:else if verifyError}
      <div class="panel error" role="alert">
        <p class="panel-title">Verification error</p>
        <p class="panel-body">{verifyError}</p>
      </div>
    {:else if verifyOutcome}
      <dl class="summary-list">
        <div>
          <dt>Checksums</dt>
          <dd class:ok={verifyOutcome.checksums_ok} class:bad={!verifyOutcome.checksums_ok}>
            {verifyOutcome.checksums_ok ? "PASS" : "FAIL"}
          </dd>
        </div>
        <div>
          <dt>Required properties</dt>
          <dd class:ok={verifyOutcome.properties_ok} class:bad={!verifyOutcome.properties_ok}>
            {verifyOutcome.properties_ok ? "PASS" : "FAIL"}
          </dd>
        </div>
      </dl>

      {#if verifyOutcome.checksum_failures.length}
        <div class="failures">
          <p class="failures-title">Checksum failures</p>
          <ul>
            {#each verifyOutcome.checksum_failures as failure (failure)}
              <li>{failure}</li>
            {/each}
          </ul>
        </div>
      {/if}

      {#if verifyOutcome.property_failures.length}
        <div class="failures">
          <p class="failures-title">Property failures</p>
          <ul>
            {#each verifyOutcome.property_failures as failure (failure)}
              <li>{failure}</li>
            {/each}
          </ul>
        </div>
      {/if}
    {/if}
  </section>

  <section class="card" aria-labelledby="report-heading">
    <h2 id="report-heading">Blinded generation report</h2>
    <p class="section-note">
      <code>{REPORT_FILE}</code> — contains configuration, counts, structure, and
      hashes. It never pairs a randomization number with an arm.
    </p>
    {#if loadingReport}
      <p class="section-note" aria-live="polite">Loading report…</p>
    {:else if reportError}
      <div class="panel error" role="alert">
        <p class="panel-title">Report unavailable</p>
        <p class="panel-body">{reportError}</p>
      </div>
    {:else if reportHtml}
      <!-- Restrictive sandbox: no scripts, no same-origin, no plugins. The
           blinded report is static HTML, so this renders fully while remaining
           CSP-safe and inert. -->
      <iframe
        class="report-frame"
        title="Blinded generation report"
        sandbox=""
        srcdoc={reportHtml}
      ></iframe>
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

  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.75rem;
  }

  .path-display {
    margin: 0 0 0.5rem;
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

  .verify-header {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
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

  .summary-list {
    margin: 0.5rem 0 0;
    padding: 0;
  }

  .summary-list div {
    display: grid;
    grid-template-columns: 12rem 1fr;
    gap: 0.5rem 1rem;
    padding: 0.35rem 0;
  }

  .summary-list div:not(:last-child) {
    border-bottom: 1px solid var(--border);
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
    font-weight: 600;
  }

  .summary-list dd.ok {
    color: #166534;
  }

  .summary-list dd.bad {
    color: var(--error-text);
  }

  .failures {
    margin-top: 0.75rem;
  }

  .failures-title {
    margin: 0 0 0.35rem;
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--error-text);
  }

  .failures ul {
    margin: 0;
    padding-left: 1.25rem;
  }

  .failures li {
    margin-bottom: 0.25rem;
    font-size: 0.875rem;
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
    .status-valid {
      color: #86efac;
      border-color: #166534;
      background-color: #052e16;
    }

    .summary-list dd.ok {
      color: #86efac;
    }
  }
</style>
