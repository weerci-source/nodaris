<script lang="ts">
    import type { Snippet } from "svelte";
    import type { HTMLButtonAttributes } from "svelte/elements";
    import Icon from "./Icon.svelte";
    import type { IconName } from "./Icon.svelte";

    type Props = HTMLButtonAttributes & {
        variant?: "primary" | "secondary" | "plain";
        size?: "sm" | "md";
        loading?: boolean;
        icon?: IconName;
        children: Snippet;
    };

    let {
        variant = "primary", size = "md", type = "button", disabled = false,
        loading = false, icon, children, ...attributes
    }: Props = $props();
</script>

<button {...attributes} {type} disabled={disabled || loading} aria-busy={loading || undefined} data-variant={variant} data-size={size}>
    {#if loading}
        <span class="spinner"><Icon name="loading" /></span>
    {:else if icon}
        <Icon name={icon} />
    {/if}
    {@render children()}
</button>

<style>
    button {
        display: inline-flex;
        align-items: center;
        justify-content: center;
        gap: var(--space-2);
        max-inline-size: 100%;
        min-block-size: var(--control-height);
        padding: var(--space-2) var(--space-3);
        border: 1px solid transparent;
        border-radius: var(--radius-md);
        background: var(--color-primary);
        color: var(--color-on-primary);
        cursor: pointer;
        font-weight: 600;
        text-align: start;
    }

    button[data-size="sm"] {
        min-block-size: var(--control-height-sm);
        padding: var(--space-1) var(--space-2);
        font-size: .875rem;
    }

    button[data-variant="secondary"], button[data-variant="plain"] {
        background: var(--color-bg);
        border-color: var(--color-border);
        color: var(--color-text);
    }

    button[data-variant="plain"] {
        justify-content: flex-start;
        background: transparent;
        border-color: transparent;
        color: inherit;
        padding-inline: var(--space-1);
    }

    button:hover:not(:disabled) { background: var(--color-primary-hover); }
    button:is([data-variant="secondary"], [data-variant="plain"]):hover:not(:disabled) {
        background: var(--color-surface-raised);
    }
    button:disabled { opacity: .7; cursor: default; }
    .spinner { display: inline-flex; animation: spin .8s linear infinite; }
    @keyframes spin { to { transform: rotate(360deg); } }
    @media (prefers-reduced-motion: reduce) { .spinner { animation: none; } }
</style>
