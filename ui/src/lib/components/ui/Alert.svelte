<script lang="ts">
    let { severity = "error", children } = $props();
</script>

<div class="alert" data-severity={severity} role="alert">
    {@render children()}
</div>

<style>
    .alert {
        padding: var(--space-3);
        border-radius: var(--radius-md);
        border: 1px solid var(--color-border);
        background: var(--color-surface);
        margin-bottom: var(--space-3);
    }
    .alert[data-severity="error"] {
        border-color: var(--color-danger);
        color: var(--color-danger);
    }
</style>
