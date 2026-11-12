<script lang="ts">
  import Scanner from "./lib/features/scanner/Scanner.svelte";
</script>

<main
  style="display: flex; flex-direction: column; height: 100vh; padding: var(--space-4);"
>
  <header
    style="display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--space-4);"
  >
    <h1 style="margin: 0;">
      Nodaris <span style="color: var(--color-primary); font-size: 0.6em;"
        >Network Scanner</span
      >
    </h1>
  </header>

  <Scanner />
</main>
