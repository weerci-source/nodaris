<script lang="ts">
    import Icon from "./Icon.svelte";

    let { message }: { message: string } = $props();
</script>

<p class="ui-muted" role="status">
    {#if message}
        <Icon name="loading" /> {message}
    {/if}
</p>

<style>
    p {
        display: flex;
        gap: var(--space-2);
        align-items: center;
        color: var(--color-primary);
        font-size: .875rem;
    }

    p:empty { position: absolute; }
</style>
