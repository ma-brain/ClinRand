<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { AboutInfo } from "$lib/types/about";

  let loading = $state(true);
  let error = $state<string | null>(null);
  let about = $state<AboutInfo | null>(null);

  async function loadAbout(): Promise<void> {
    loading = true;
    error = null;
    try {
      about = await invoke<AboutInfo>("get_about");
    } catch (err) {
      about = null;
      error =
        err instanceof Error
          ? err.message
          : "Unable to load version information from the application.";
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void loadAbout();
  });
</script>

<h1>About</h1>

{#if loading}
  <p class="status">Loading version information…</p>
{:else if error}
  <div class="error-panel" role="alert">
    <p class="error-title">Could not load about information</p>
    <p class="error-message">{error}</p>
    <button type="button" onclick={() => void loadAbout()}>Retry</button>
  </div>
{:else if about}
  <section class="about-section" aria-labelledby="versions-heading">
    <h2 id="versions-heading">Version information</h2>
    <dl class="version-list">
      <div>
        <dt>Engine version</dt>
        <dd>{about.engine_version}</dd>
      </div>
      <div>
        <dt>Algorithm version</dt>
        <dd>{about.algo_version}</dd>
      </div>
      <div>
        <dt>RNG algorithm</dt>
        <dd>{about.rng_algorithm}</dd>
      </div>
      <div>
        <dt>RNG crate</dt>
        <dd>{about.rng_crate}</dd>
      </div>
      <div>
        <dt>RNG crate version</dt>
        <dd>{about.rng_crate_version}</dd>
      </div>
    </dl>
  </section>

  <section class="about-section" aria-labelledby="disclaimer-heading">
    <h2 id="disclaimer-heading">Scope disclaimer</h2>
    <p class="disclaimer">{about.disclaimer}</p>
  </section>
{/if}

<style>
  h1 {
    margin: 0 0 1.5rem;
    font-size: 1.5rem;
    font-weight: 600;
  }

  h2 {
    margin: 0 0 0.75rem;
    font-size: 1rem;
    font-weight: 600;
  }

  .status {
    margin: 0;
    color: var(--text-muted);
  }

  .about-section {
    margin-bottom: 2rem;
  }

  .version-list {
    margin: 0;
    padding: 1rem 1.25rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background-color: var(--surface);
  }

  .version-list div {
    display: grid;
    grid-template-columns: 11rem 1fr;
    gap: 0.5rem 1rem;
    padding: 0.35rem 0;
  }

  .version-list div:not(:last-child) {
    border-bottom: 1px solid var(--border);
    padding-bottom: 0.5rem;
    margin-bottom: 0.25rem;
  }

  .version-list dt {
    margin: 0;
    color: var(--text-muted);
    font-weight: 400;
  }

  .version-list dd {
    margin: 0;
    font-family: ui-monospace, "Cascadia Code", "Source Code Pro", Menlo, monospace;
    font-size: 0.9375rem;
  }

  .disclaimer {
    margin: 0;
    padding: 1rem 1.25rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background-color: var(--surface);
    color: var(--text);
  }

  .error-panel {
    padding: 1rem 1.25rem;
    border: 1px solid var(--error-border);
    border-radius: 6px;
    background-color: var(--error-bg);
    color: var(--error-text);
  }

  .error-title {
    margin: 0 0 0.25rem;
    font-weight: 600;
  }

  .error-message {
    margin: 0 0 0.75rem;
  }

  button {
    padding: 0.375rem 0.75rem;
    border: 1px solid var(--error-border);
    border-radius: 4px;
    background-color: var(--surface);
    color: var(--text);
    font: inherit;
    cursor: pointer;
  }

  button:hover {
    border-color: var(--accent);
    color: var(--accent);
  }
</style>
