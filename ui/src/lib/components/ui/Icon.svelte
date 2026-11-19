<script module lang="ts">
    export type IconName = "wifi" | "refresh" | "loading" | "check" | "info" | "warning" | "error" | "chevron-right" | "chevron-down";
</script>

<script lang="ts">
    let { name, size = 16 }: { name: IconName; size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
    {#if name === "wifi"}
        <path d="M3 9a14 14 0 0 1 18 0M6 13a9.5 9.5 0 0 1 12 0m-8.5 4a4 4 0 0 1 5 0" />
        <circle cx="12" cy="20" r="1" fill="currentColor" stroke="none" />
    {:else if name === "refresh"}
        <path d="M20 5v5h-5m5 0a8 8 0 1 0-.8 6" />
    {:else if name === "loading"}
        <path d="M12 4a8 8 0 1 1-8 8" />
    {:else if name === "check"}
        <path d="m5 12 4 4L19 6" />
    {:else if name === "chevron-right"}
        <path d="m10 6 6 6-6 6" />
    {:else if name === "chevron-down"}
        <path d="m6 10 6 6 6-6" />
    {:else if name === "warning"}
        <path d="M12 3 2.5 20h19L12 3Zm0 6v4m0 3.5h.01" />
    {:else}
        <circle cx="12" cy="12" r="9" />
        {#if name === "info"}
            <path d="M12 11v5m0-8h.01" />
        {:else}
            <path d="m9 9 6 6m0-6-6 6" />
        {/if}
    {/if}
</svg>

<style>
    svg { flex: none; vertical-align: middle; }
</style>
