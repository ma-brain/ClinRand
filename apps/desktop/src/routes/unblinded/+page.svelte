<script lang="ts">
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { exists, readTextFile, writeTextFile } from "@tauri-apps/plugin-fs";

  const REPORT_FILE = "unblinded-report.html";
  const ACCESS_LOG_FILE = "access-log.txt";

  let lastPackageDir = $state<string | null>(null);

  // The directory chosen in the picker but not yet revealed. Selecting a folder
  // never reveals unblinded material; it only arms the confirmation step.
  let pendingDir = $state<string | null>(null);
  let confirmOpen = $state(false);

  let pickerError = $state<string | null>(null);

  // The directory whose unblinded report is currently revealed.
  let revealedDir = $state<string | null>(null);
  let revealing = $state(false);
  let reportHtml = $state<string | null>(null);
  let revealError = $state<string | null>(null);

  // Monotonic request token. Each reveal increments it; the async helper only
  // commits its results if its captured token is still current, so an earlier,
  // slower reveal can never overwrite a later selection's state.
  let loadToken = 0;

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

  /** ISO-8601 UTC timestamp to whole seconds, e.g. `2026-09-15T15:04:05Z`. */
  function utcTimestamp(): string {
    return new Date().toISOString().replace(/\.\d{3}Z$/, "Z");
  }

  /**
   * Append the mandatory access-log entry inside the package directory.
   * Throws on any failure so the caller can refuse to reveal content.
   */
  async function appendAccessLog(dir: string): Promise<void> {
    const path = joinPath(dir, ACCESS_LOG_FILE);
    const line = `${utcTimestamp()} unblinded view opened\n`;
    // `append` creates the file when it does not yet exist.
    await writeTextFile(path, line, { append: true });
  }

  async function openPicker(): Promise<void> {
    pickerError = null;
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Select ClinRand package directory",
        defaultPath: lastPackageDir ?? undefined,
      });
      if (typeof selected === "string") {
        // Arm the confirmation step; do not reveal anything yet.
        pendingDir = selected;
        confirmOpen = true;
      }
    } catch (err) {
      pickerError =
        err instanceof Error
          ? err.message
          : "Could not open the folder picker.";
    }
  }

  function cancelConfirm(): void {
    confirmOpen = false;
    pendingDir = null;
  }

  async function confirmReveal(): Promise<void> {
    const dir = pendingDir;
    if (dir === null) return;
    confirmOpen = false;

    const token = ++loadToken;
    revealing = true;
    revealedDir = dir;
    reportHtml = null;
    revealError = null;

    // The access-log write is mandatory: if it fails we must not reveal the
    // unblinded content, because the audit trail would be incomplete.
    try {
      await appendAccessLog(dir);
    } catch (err) {
      if (token !== loadToken) return;
      revealedDir = null;
      reportHtml = null;
      revealError =
        (err instanceof Error ? err.message : String(err)) +
        "\n\nThe access log could not be written, so the unblinded content was not opened. The audit entry is mandatory.";
      revealing = false;
      pendingDir = null;
      return;
    }

    try {
      const path = joinPath(dir, REPORT_FILE);
      const present = await exists(path);
      if (token !== loadToken) return;
      if (!present) {
        reportHtml = null;
        revealError = `This folder does not contain ${REPORT_FILE}. Confirm you selected a ClinRand package directory.`;
        return;
      }
      const html = await readTextFile(path);
      if (token !== loadToken) return;
      reportHtml = html;
      revealError = null;
    } catch (err) {
      if (token !== loadToken) return;
      reportHtml = null;
      revealError =
        err instanceof Error
          ? err.message
          : "The unblinded report could not be read.";
    } finally {
      if (token === loadToken) {
        revealing = false;
        pendingDir = null;
      }
    }
  }
</script>

<h1>Unblinded view</h1>

<p class="intro">
  This screen reveals randomization-number ↔ arm assignments and is for the
  unblinded statistician only. Opening it writes a permanent audit entry to the
  package's access log. Nothing is revealed until you explicitly confirm.
</p>

<section class="card" aria-labelledby="open-heading">
  <h2 id="open-heading">Open package</h2>
  {#if lastPackageDir}
    <p class="section-note">Last generated package this session:</p>
    <p class="path-display"><code>{lastPackageDir}</code></p>
    <p class="section-note">
      Select it (or any package directory) in the folder picker.
    </p>
  {:else}
    <p class="section-note">
      Choose a package directory produced by the generate screen.
    </p>
  {/if}
  <div class="row">
    <button
      type="button"
      class="primary"
      onclick={openPicker}
      disabled={revealing}
    >
      {#if revealing}
        Opening…
      {:else}
        {lastPackageDir ? "Open a package…" : "Choose package folder…"}
      {/if}
    </button>
    <p class="path-display" aria-live="polite">
      {#if revealedDir}
        <code>{revealedDir}</code>
      {:else}
        <span class="path-placeholder">No package opened</span>
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

{#if revealError}
  <section class="card" aria-labelledby="reveal-error-heading">
    <div class="panel error" role="alert">
      <p class="panel-title" id="reveal-error-heading">Unblinded view not opened</p>
      <p class="panel-body">{revealError}</p>
    </div>
  </section>
{/if}

{#if reportHtml}
  <section class="card" aria-labelledby="report-heading">
    <div class="report-header">
      <h2 id="report-heading">Unblinded report</h2>
      <span class="status status-unblinded" role="status">Unblinded — restricted</span>
    </div>
    <p class="section-note">
      <code>{REPORT_FILE}</code> — contains the full list, pairing each
      randomization number with its arm. The seed is never shown.
    </p>
    <!-- Restrictive sandbox: no scripts, no same-origin, no plugins. The
         unblinded report is static HTML, so this renders fully while remaining
         CSP-safe and inert. -->
    <iframe
      class="report-frame"
      title="Unblinded report"
      sandbox=""
      srcdoc={reportHtml}
    ></iframe>
  </section>
{/if}

{#if confirmOpen}
  <div
    class="modal-backdrop"
    role="dialog"
    aria-modal="true"
    aria-labelledby="confirm-title"
    aria-describedby="confirm-body"
  >
    <div class="modal">
      <h2 id="confirm-title" class="modal-title">Reveal unblinded assignments?</h2>
      <div id="confirm-body" class="modal-body">
        <p>
          You are about to view the <strong>unblinded report</strong>, which
          pairs every randomization number with its assigned treatment arm. This
          material breaks the blind.
        </p>
        <p>Confirming will:</p>
        <ul>
          <li>
            write a permanent audit entry to
            <code>{ACCESS_LOG_FILE}</code> inside the package folder, and
          </li>
          <li>display the assignments on screen.</li>
        </ul>
        <p class="modal-path">
          <code>{pendingDir}</code>
        </p>
      </div>
      <div class="modal-actions">
        <button type="button" class="secondary" onclick={cancelConfirm}>
          Cancel
        </button>
        <button type="button" class="danger" onclick={confirmReveal}>
          Reveal unblinded assignments
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

  button.secondary {
    padding: 0.5rem 1rem;
    border: 1px solid var(--border);
    background-color: transparent;
    color: var(--text);
    font-size: 0.9375rem;
    font-weight: 500;
  }

  button.secondary:hover:not(:disabled) {
    background-color: var(--surface-hover, rgba(127, 127, 127, 0.1));
  }

  button.danger {
    padding: 0.5rem 1rem;
    border: 1px solid var(--error-border);
    background-color: var(--error-text);
    color: #fff;
    font-size: 0.9375rem;
    font-weight: 600;
  }

  button.danger:hover:not(:disabled) {
    filter: brightness(0.92);
  }

  .report-header {
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

  .status-unblinded {
    color: var(--error-text);
    border-color: var(--error-border);
    background-color: var(--error-bg);
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
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1.5rem;
    background-color: rgba(0, 0, 0, 0.5);
    z-index: 50;
  }

  .modal {
    width: min(32rem, 100%);
    padding: 1.5rem;
    border: 1px solid var(--border);
    border-radius: 8px;
    background-color: var(--surface);
    box-shadow: 0 10px 40px rgba(0, 0, 0, 0.35);
  }

  .modal-title {
    margin: 0 0 0.75rem;
    font-size: 1.125rem;
    font-weight: 600;
    color: var(--error-text);
  }

  .modal-body {
    font-size: 0.9375rem;
    color: var(--text);
  }

  .modal-body p {
    margin: 0 0 0.75rem;
  }

  .modal-body ul {
    margin: 0 0 0.75rem;
    padding-left: 1.25rem;
  }

  .modal-body li {
    margin-bottom: 0.25rem;
  }

  .modal-path {
    word-break: break-all;
  }

  .modal-actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 0.75rem;
    margin-top: 1.25rem;
  }
</style>
