<script lang="ts">
    import type { Snippet } from "svelte";
    import type { ApiError } from "../../i18n";

    let { severity = "error", children }: { severity?: ApiError["severity"]; children: Snippet } = $props();
</script>

<div class="alert" data-severity={severity} role="alert">
    {@render children()}
</div>

<style>
    .alert {
        --alert-color: var(--color-primary);
        padding: var(--space-3);
        border-radius: var(--radius-md);
        border: 1px solid var(--color-border);
        border-inline-start: 3px solid var(--alert-color);
        background: var(--color-surface);
    }
    .alert[data-severity="warning"] { --alert-color: var(--color-warning); }
    .alert[data-severity="error"], .alert[data-severity="critical"] { --alert-color: var(--color-danger); }
</style>
