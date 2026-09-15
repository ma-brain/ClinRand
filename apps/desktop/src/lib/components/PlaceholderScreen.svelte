<script lang="ts">
  let { title }: { title: string } = $props();
</script>

<h1>{title}</h1>
<p class="placeholder-note">
  This screen will be implemented in a later Phase 7 task.
</p>

<style>
  h1 {
    margin: 0 0 0.75rem;
    font-size: 1.5rem;
    font-weight: 600;
  }

  .placeholder-note {
    margin: 0;
    color: var(--text-muted);
  }
</style>
