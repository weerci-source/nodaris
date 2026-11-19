<script module lang="ts">
    import type { IconName } from "./Icon.svelte";

    export type TabOption<T extends string> = { value: T; label: string; icon?: IconName };
</script>

<script lang="ts" generics="T extends string">
    import Icon from "./Icon.svelte";

    let { group, label, tabs, active, onSelect, variant = "section" }: {
        group: string;
        label: string;
        tabs: readonly TabOption<T>[];
        active: T;
        onSelect: (value: T) => void;
        variant?: "workspace" | "section";
    } = $props();

    let list: HTMLDivElement;

    function navigate(event: KeyboardEvent, index: number) {
        const rtl = getComputedStyle(list).direction === "rtl";
        let next: number;
        switch (event.key) {
            case "ArrowRight": next = index + (rtl ? -1 : 1); break;
            case "ArrowLeft": next = index + (rtl ? 1 : -1); break;
            case "Home": next = 0; break;
            case "End": next = tabs.length - 1; break;
            default: return;
        }
        event.preventDefault();
        next = (next + tabs.length) % tabs.length;
        onSelect(tabs[next].value);
        list.querySelectorAll<HTMLButtonElement>("[role=tab]")[next]?.focus();
    }
</script>

<div class="tab-list" data-variant={variant} role="tablist" aria-label={label} bind:this={list}>
    {#each tabs as tab, index (tab.value)}
        <button class="tab" type="button" role="tab" id={`${group}-tab-${tab.value}`}
            aria-controls={`${group}-panel-${tab.value}`} aria-selected={active === tab.value}
            tabindex={active === tab.value ? 0 : -1}
            onclick={() => onSelect(tab.value)} onkeydown={(event) => navigate(event, index)}>
            {#if tab.icon}<Icon name={tab.icon} size={18} />{/if}
            <span>{tab.label}</span>
        </button>
    {/each}
</div>
