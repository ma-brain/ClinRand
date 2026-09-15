<script lang="ts">
  import { page } from "$app/state";
  import { navItems } from "$lib/nav";

  let { children } = $props();
</script>

<div class="app-shell">
  <nav class="sidebar" aria-label="Main navigation">
    <div class="brand">
      <a href="/config" class="brand-link">ClinRand</a>
    </div>
    <ul class="nav-list">
      {#each navItems as item (item.href)}
        <li>
          <a
            href={item.href}
            class:active={page.url.pathname === item.href}
            aria-current={page.url.pathname === item.href ? "page" : undefined}
          >
            {item.label}
          </a>
        </li>
      {/each}
    </ul>
  </nav>

  <main class="content">
    {@render children()}
  </main>
</div>

<style>
  :global(:root) {
    --text: #1a1a1a;
    --text-muted: #555;
    --surface: #fff;
    --surface-muted: #f6f6f6;
    --border: #d0d0d0;
    --accent: #2c5282;
    --accent-hover: #1a365d;
    --error-bg: #fef2f2;
    --error-border: #fecaca;
    --error-text: #991b1b;

    font-family: system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
    font-size: 16px;
    line-height: 1.5;
    color: var(--text);
    background-color: var(--surface-muted);
  }

  :global(*, *::before, *::after) {
    box-sizing: border-box;
  }

  :global(body) {
    margin: 0;
  }

  .app-shell {
    display: flex;
    min-height: 100vh;
  }

  .sidebar {
    flex: 0 0 14rem;
    padding: 1.25rem 0;
    border-right: 1px solid var(--border);
    background-color: var(--surface);
  }

  .brand {
    padding: 0 1.25rem 1rem;
    border-bottom: 1px solid var(--border);
    margin-bottom: 0.75rem;
  }

  .brand-link {
    font-size: 1.125rem;
    font-weight: 600;
    color: var(--text);
    text-decoration: none;
  }

  .brand-link:hover {
    color: var(--accent);
  }

  .nav-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .nav-list a {
    display: block;
    padding: 0.5rem 1.25rem;
    color: var(--text-muted);
    text-decoration: none;
    border-left: 3px solid transparent;
  }

  .nav-list a:hover {
    color: var(--text);
    background-color: var(--surface-muted);
  }

  .nav-list a.active {
    color: var(--accent);
    font-weight: 500;
    border-left-color: var(--accent);
    background-color: var(--surface-muted);
  }

  .content {
    flex: 1;
    padding: 2rem 2.5rem;
    max-width: 52rem;
  }

  @media (prefers-color-scheme: dark) {
    :global(:root) {
      --text: #f0f0f0;
      --text-muted: #aaa;
      --surface: #2a2a2a;
      --surface-muted: #1e1e1e;
      --border: #3a3a3a;
      --accent: #63b3ed;
      --accent-hover: #90cdf4;
      --error-bg: #3b1212;
      --error-border: #7f1d1d;
      --error-text: #fecaca;
    }
  }
</style>
