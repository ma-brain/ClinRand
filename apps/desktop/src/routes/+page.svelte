<script lang="ts">
  // Placeholder home page. Real screens (config, preview, generate, verify,
  // unblinded view, validation, about) arrive in later Phase 7 tasks. No Tauri
  // commands are invoked yet — this task is scaffold only.
  const screens = [
    "Config builder",
    "Structure preview (blinded)",
    "Generate",
    "Package viewer & verify",
    "Unblinded view",
    "Validation",
    "About",
  ];
</script>

<main class="container">
  <h1>ClinRand</h1>
  <p class="tagline">Offline clinical-trial randomization list generator</p>

  <section class="scaffold-note">
    <p>Desktop application scaffold. Screens are implemented in later tasks.</p>
    <ul>
      {#each screens as screen}
        <li>{screen}</li>
      {/each}
    </ul>
  </section>
</main>

<style>
  :root {
    font-family: system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
    font-size: 16px;
    line-height: 1.5;
    color: #1a1a1a;
    background-color: #f6f6f6;
  }

  .container {
    margin: 0 auto;
    max-width: 40rem;
    padding: 3rem 1.5rem;
  }

  h1 {
    margin-bottom: 0.25rem;
    font-size: 2rem;
  }

  .tagline {
    margin-top: 0;
    color: #555;
  }

  .scaffold-note {
    margin-top: 2rem;
    padding: 1rem 1.25rem;
    border: 1px solid #d0d0d0;
    border-radius: 8px;
    background-color: #fff;
  }

  .scaffold-note ul {
    margin: 0.5rem 0 0;
    padding-left: 1.25rem;
  }

  @media (prefers-color-scheme: dark) {
    :root {
      color: #f0f0f0;
      background-color: #1e1e1e;
    }

    .tagline {
      color: #aaa;
    }

    .scaffold-note {
      border-color: #3a3a3a;
      background-color: #2a2a2a;
    }
  }
</style>
