<script lang="ts">
    import Scanner from "./lib/features/scanner/Scanner.svelte";
    import { isTauri } from "@tauri-apps/api/core";
    import EthernetPanel from "./lib/features/ethernet/EthernetPanel.svelte";
    import RadioDiagnostics from "./lib/features/radio/RadioDiagnostics.svelte";
    import SessionPanel from "./lib/features/session/SessionPanel.svelte";
    import Icon from "./lib/components/ui/Icon.svelte";
    import { t } from "./lib/i18n";
</script>

<header class="topbar">
    <div class="frame brand">
        <span class="brand-mark" aria-hidden="true">
            <Icon name="wifi" size={18} />
        </span>
        <strong>Nodaris</strong>
        <span class="subtitle">{t("app_subtitle")}</span>
        <nav class="section-links" aria-label={t("app_sections")}>
            <a href="#diagnostic-session">{t("session_title")}</a>
            <a href="#ethernet">{t("ethernet_title")}</a>
        </nav>
    </div>
</header>

<main class="frame">
    <Scanner />
    <SessionPanel />
    <EthernetPanel />
    {#if !isTauri()}
        <RadioDiagnostics />
    {/if}
</main>

<style>
    .frame { width: min(100%, 70rem); margin-inline: auto; padding-inline: clamp(var(--space-3), 4vw, var(--space-5)); }
    .topbar { flex: none; border-bottom: 1px solid var(--color-border); background: var(--color-surface); }
    .brand { min-height: 3.75rem; display: flex; align-items: center; gap: var(--space-2); }
    .brand-mark { display: grid; place-items: center; width: 2rem; height: 2rem; background: var(--color-bg); color: var(--color-primary); border: 1px solid var(--color-border); border-radius: var(--radius-md); }
    .subtitle { color: var(--color-text-muted); font-size: .75rem; }
    @media (max-width: 480px) { .subtitle { display: none; } }
    main {
        flex: 1;
        min-height: 0;
        min-width: 0;
        overflow-y: auto;
        padding-block: clamp(var(--space-3), 4vw, var(--space-5));
    }
</style>
