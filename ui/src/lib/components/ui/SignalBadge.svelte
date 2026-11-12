<script lang="ts">
    let { signal } = $props();

    let bars = $derived(
        signal >= 80
            ? "▂▄▆█"
            : signal >= 60
              ? "▂▄▆_"
              : signal >= 40
                ? "▂▄__"
                : signal >= 20
                  ? "▂___"
                  : "____",
    );
    let color = $derived(
        signal > 60
            ? "var(--color-success)"
            : signal > 40
              ? "var(--color-warning)"
              : "var(--color-danger)",
    );
</script>

<div style="display: flex; align-items: center; gap: 8px;">
    <span style="font-family: monospace; letter-spacing: 2px; color: {color};"
        >{bars}</span
    >
    <span style="color: var(--color-text-muted); font-size: 0.85em;"
        >{signal}</span
    >
</div>
