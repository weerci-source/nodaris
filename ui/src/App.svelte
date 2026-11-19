<script lang="ts">
    import { isTauri } from "@tauri-apps/api/core";
    import { onMount, tick } from "svelte";
    import Icon from "./lib/components/ui/Icon.svelte";
    import TabList, { type TabOption } from "./lib/components/ui/TabList.svelte";
    import TabPanel from "./lib/components/ui/TabPanel.svelte";
    import EthernetPanel from "./lib/features/ethernet/EthernetPanel.svelte";
    import HistoryPanel from "./lib/features/history/HistoryPanel.svelte";
    import type { HistoryEntry } from "./lib/features/history/history-api";
    import RadioDiagnostics from "./lib/features/radio/RadioDiagnostics.svelte";
    import Scanner from "./lib/features/scanner/Scanner.svelte";
    import SessionPanel from "./lib/features/session/SessionPanel.svelte";
    import SettingsPanel from "./lib/features/settings/SettingsPanel.svelte";
    import { t } from "./lib/i18n";

    type WorkspaceTab = "wifi" | "session" | "ethernet" | "history" | "settings";

    // Previously published section anchors can still open their corresponding tab.
    const bookmarks: Record<string, WorkspaceTab> = {
        "#diagnostic-session": "session",
        "#ethernet": "ethernet",
        "#diagnostic-history": "history",
    };
    const group = "workspace";
    let tab = $state<WorkspaceTab>(bookmarks[window.location.hash] ?? "wifi");
    let tabs = $derived<TabOption<WorkspaceTab>[]>([
        { value: "wifi", icon: "wifi", label: t("workspace_wifi") },
        { value: "session", icon: "session", label: t("workspace_session") },
        { value: "ethernet", icon: "ethernet", label: t("ethernet_title") },
        { value: "history", icon: "history", label: t("history_title") },
        { value: "settings", icon: "settings", label: t("settings_title") },
    ]);
    let viewport: HTMLElement;
    let session: SessionPanel;
    let ethernet: EthernetPanel;
    let history: HistoryPanel;

    function selectTab(value: WorkspaceTab) {
        if (value === tab) return;
        tab = value;
        viewport.scrollTop = 0;
    }

    function refreshHistory() {
        void history?.refresh();
    }

    async function repeat(entry: HistoryEntry) {
        return entry.kind === "session"
            ? await session.prepareRepeat(entry.report.interface, entry.target, () => selectTab("session"))
            : await ethernet.prepareRepeat(entry.report.link.interface, entry.target, () => selectTab("ethernet"));
    }

    async function openHistory() {
        selectTab("history");
        await tick();
        document.getElementById("workspace-tab-history")?.focus();
    }

    onMount(() => {
        function openBookmark() {
            const bookmarked = bookmarks[window.location.hash];
            if (bookmarked) selectTab(bookmarked);
        }
        window.addEventListener("hashchange", openBookmark);
        return () => window.removeEventListener("hashchange", openBookmark);
    });
</script>

<header class="workspace-header">
    <div class="workspace-frame workspace-top">
        <div class="workspace-brand">
            <span class="workspace-mark" aria-hidden="true"><Icon name="wifi" size={20} /></span>
            <strong>Nodaris</strong>
        </div>
        <TabList {group} label={t("app_sections")} {tabs} active={tab} onSelect={selectTab} variant="workspace" />
    </div>
</header>

<main class="workspace-main" bind:this={viewport}>
    <div class="workspace-frame">
        <TabPanel {group} value="wifi" active={tab === "wifi"}>
            <Scanner />
            {#if !isTauri()}<RadioDiagnostics />{/if}
        </TabPanel>
        <TabPanel {group} value="session" active={tab === "session"}>
            <SessionPanel bind:this={session} onSaved={refreshHistory} />
        </TabPanel>
        <TabPanel {group} value="ethernet" active={tab === "ethernet"}>
            <EthernetPanel bind:this={ethernet} onSaved={refreshHistory} />
        </TabPanel>
        <TabPanel {group} value="history" active={tab === "history"}>
            <HistoryPanel bind:this={history} onRepeat={repeat} />
        </TabPanel>
        <TabPanel {group} value="settings" active={tab === "settings"}>
            <SettingsPanel onOpenHistory={openHistory} />
        </TabPanel>
    </div>
</main>
