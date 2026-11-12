<script lang="ts">
  import { onMount } from "svelte";

  let status = { name: "Loading...", version: "", status: "" };

  onMount(async () => {
    try {
      const res = await fetch("http://localhost:8080/api/status");
      status = await res.json();
    } catch (e) {
      console.error("Server not running", e);
    }
  });
</script>

<main
  style="background: blue; color: white; min-height: 100vh; padding: 2rem; font-family: sans-serif;"
>
  <h1>
    🛡️ {status.name}
    <span style="font-size: 0.5em; color: gray;">v{status.version}</span>
  </h1>
  <p>Status: <strong style="color: #00d2ff;">{status.status}</strong></p>
</main>
