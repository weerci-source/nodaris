<script lang="ts">
    let { value, missingLabel }: { value: number | null; missingLabel: string } = $props();
</script>

{#if value === null}
    <span class="missing">{missingLabel}</span>
{:else}
    <span class="reading">
        <span class="track" aria-hidden="true"><span class="fill" style:width={`${value}%`}></span></span>
        <strong>{value}%</strong>
    </span>
{/if}

<style>
    .reading { display: inline-flex; align-items: center; gap: var(--space-2); white-space: nowrap; }
    .track { width: clamp(1.75rem, 7vw, 4.25rem); height: var(--space-1); border-radius: 99px; background: var(--color-border); overflow: hidden; }
    .fill { display: block; height: 100%; background: var(--color-primary); border-radius: inherit; }
    strong { font-size: .875rem; font-variant-numeric: tabular-nums; }
    .missing { color: var(--color-text-muted); font-size: .875rem; }
</style>
