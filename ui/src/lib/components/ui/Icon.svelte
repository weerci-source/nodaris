<script module lang="ts">
    export type IconName = "wifi" | "refresh" | "loading" | "check" | "info" | "warning" | "error"
        | "chevron-right" | "chevron-down" | "chevron-left" | "more" | "session" | "ethernet" | "history" | "settings" | "sun" | "moon" | "monitor" | "search" | "close";
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
    {:else if name === "search"}
        <circle cx="10.5" cy="10.5" r="5.5" />
        <path d="m15 15 4 4" />
    {:else if name === "close"}
        <path d="m6 6 12 12M18 6 6 18" />
    {:else if name === "check"}
        <path d="m5 12 4 4L19 6" />
    {:else if name === "chevron-right"}
        <path d="m10 6 6 6-6 6" />
    {:else if name === "chevron-left"}
        <path d="m14 6-6 6 6 6" />
    {:else if name === "more"}
        <path d="M5 12h.01m6.99 0h.01m6.99 0h.01" stroke-width="3" />
    {:else if name === "chevron-down"}
        <path d="m6 10 6 6 6-6" />
    {:else if name === "session"}
        <path d="M9 5 6 8l-3-3m3 3v9a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2v-2m-3 0 3-3 3 3M6 5a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v4" />
    {:else if name === "ethernet"}
        <path d="M4 5h16v11h-4v3H8v-3H4zM8 9v3m4-3v3m4-3v3" />
    {:else if name === "history"}
        <path d="M3 4v5h5m-5 0a9 9 0 1 1 .65 8m8.35-9v5l3 2" />
    {:else if name === "settings"}
        <path d="M4 6h4m4 0h8M4 12h9m4 0h3M4 18h2m4 0h10M8 4v4m5 2v4m-7 2v4" />
    {:else if name === "sun"}
        <circle cx="12" cy="12" r="4" />
        <path d="M12 2v2m0 16v2M2 12h2m16 0h2M5 5l1.5 1.5m11 11L19 19m0-14-1.5 1.5m-11 11L5 19" />
    {:else if name === "moon"}
        <path d="M20.5 15a8.6 8.6 0 0 1-11.5-11.5A8.7 8.7 0 1 0 20.5 15Z" />
    {:else if name === "monitor"}
        <rect x="3" y="4" width="18" height="13" rx="2" />
        <path d="M12 17v4m-3 0h6" />
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
