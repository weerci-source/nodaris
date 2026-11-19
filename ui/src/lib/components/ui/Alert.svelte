<script lang="ts">
    import type { Snippet } from "svelte";
    import Icon from "./Icon.svelte";

    let { severity = "error", children }: {
        severity?: "info" | "warning" | "error" | "critical";
        children: Snippet;
    } = $props();

    let urgent = $derived(severity === "error" || severity === "critical");
</script>

<div class="alert" data-severity={severity} role={urgent ? "alert" : "status"}>
    <span class="icon"><Icon name={urgent ? "error" : severity === "warning" ? "warning" : "info"} /></span>
    <div>{@render children()}</div>
</div>

<style>
    .alert {
        --alert-color: var(--color-primary);
        display: flex;
        align-items: start;
        gap: var(--space-2);
        padding: var(--space-3);
        border-radius: var(--radius-md);
        border: 1px solid var(--color-border);
        border-inline-start: var(--space-1) solid var(--alert-color);
        background: var(--color-surface);
        overflow-wrap: anywhere;
    }
    .icon { color: var(--alert-color); }
    .alert[data-severity="warning"] { --alert-color: var(--color-warning); }
    .alert[data-severity="error"], .alert[data-severity="critical"] { --alert-color: var(--color-danger); }
</style>
