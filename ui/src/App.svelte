<script lang="ts">
    import Scanner from "./lib/features/scanner/Scanner.svelte";
    import { isTauri } from "@tauri-apps/api/core";
    import RadioDiagnostics from "./lib/features/radio/RadioDiagnostics.svelte";
    import { t } from "./lib/i18n";
</script>

<header class="topbar">
    <div class="frame brand">
        <span class="brand-mark" aria-hidden="true">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
                <path d="M3 9a14 14 0 0 1 18 0M6 13a9.5 9.5 0 0 1 12 0m-8.5 4a4 4 0 0 1 5 0" stroke="currentColor" stroke-width="1.7" stroke-linecap="round"/>
                <circle cx="12" cy="20" r="1" fill="currentColor"/>
            </svg>
        </span>
        <strong>Nodaris</strong>
        <span class="subtitle">{t("app_subtitle")}</span>
    </div>
</header>

<main class="frame">
    <Scanner />
    {#if !isTauri()}
        <RadioDiagnostics />
    {/if}
</main>

<style>
    .frame { width: min(100%, 70rem); margin-inline: auto; padding-inline: clamp(var(--space-3), 4vw, var(--space-5)); }
    .topbar { border-bottom: 1px solid var(--color-border); background: var(--color-surface); }
    .brand { min-height: 3.75rem; display: flex; align-items: center; gap: var(--space-2); }
    .brand-mark { display: grid; place-items: center; width: 2rem; height: 2rem; background: var(--color-bg); color: var(--color-primary); border: 1px solid var(--color-border); border-radius: var(--radius-md); }
    .subtitle { color: var(--color-text-muted); font-size: .75rem; }
    main { padding-block: clamp(var(--space-3), 4vw, var(--space-5)); }
</style>
