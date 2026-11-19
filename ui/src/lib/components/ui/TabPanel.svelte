<script lang="ts">
    import type { Snippet } from "svelte";

    let { group, value, active, children }: {
        group: string;
        value: string;
        active: boolean;
        children: Snippet;
    } = $props();
</script>

<div class="tab-panel" role="tabpanel" id={`${group}-panel-${value}`}
    aria-labelledby={`${group}-tab-${value}`} tabindex="0" hidden={!active}>
    {@render children()}
</div>
