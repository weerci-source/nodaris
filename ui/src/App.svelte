<script lang="ts">
    import { isTauri } from "@tauri-apps/api/core";
    import { onMount, tick } from "svelte";
    import Icon from "./lib/components/ui/Icon.svelte";
    import TabList, { type TabOption } from "./lib/components/ui/TabList.svelte";
    import TabPanel from "./lib/components/ui/TabPanel.svelte";
    import CommandPalette, { type Command } from "./lib/features/commands/CommandPalette.svelte";
    import EthernetPanel from "./lib/features/ethernet/EthernetPanel.svelte";
    import HistoryPanel from "./lib/features/history/HistoryPanel.svelte";
    import { readHistory, type HistoryEntry } from "./lib/features/history/history-api";
    import RadioDiagnostics from "./lib/features/radio/RadioDiagnostics.svelte";
    import Scanner from "./lib/features/scanner/Scanner.svelte";
    import SessionPanel from "./lib/features/session/SessionPanel.svelte";
    import SettingsPanel from "./lib/features/settings/SettingsPanel.svelte";
    import { t, type ApiError } from "./lib/i18n";

    type WorkspaceTab = "wifi" | "session" | "ethernet" | "history" | "settings";
    type WorkspaceCommand = WorkspaceTab | "appearance" | "repeat-latest";

    // Previously published section anchors can still open their corresponding tab.
    const bookmarks: Record<string, WorkspaceTab> = {
        "#diagnostic-session": "session",
        "#ethernet": "ethernet",
        "#diagnostic-history": "history",
    };
    const group = "workspace";
    const desktop = isTauri();
    const keywords: Record<WorkspaceTab, string> = {
        wifi: "wifi wi-fi wireless scan сканирование радиодиагностика беспроводная сеть",
        session: "session diagnostics сеанс диагностика",
        ethernet: "ethernet wired link проводная сеть линк пробы",
        history: "history reports история отчеты замеры",
        settings: "settings preferences настройки параметры",
    };
    let tab = $state<WorkspaceTab>(bookmarks[window.location.hash] ?? "wifi");
    let tabs = $derived<TabOption<WorkspaceTab>[]>([
        { value: "wifi", icon: "wifi", label: t("workspace_wifi") },
        { value: "session", icon: "session", label: t("workspace_session") },
        { value: "ethernet", icon: "ethernet", label: t("ethernet_title") },
        { value: "history", icon: "history", label: t("history_title") },
        { value: "settings", icon: "settings", label: t("settings_title") },
    ]);
    let commands = $derived<Command<WorkspaceCommand>[]>([
        ...tabs.map(({ value, label, icon }) => ({ id: value, label, icon, keywords: keywords[value] })),
        {
            id: "appearance", label: t("settings_appearance"), icon: "sun",
            keywords: "appearance theme dark light language внешний вид тема светлая темная язык",
        },
        {
            id: "repeat-latest", label: t("commands_repeat_latest"), icon: "refresh",
            keywords: "repeat rerun latest measurement повтор повторить последнего замера",
        },
    ]);
    let viewport: HTMLElement;
    let session: SessionPanel;
    let ethernet: EthernetPanel;
    let history: HistoryPanel;
    let settings: SettingsPanel;

    function selectTab(value: WorkspaceTab) {
        if (value === tab) return;
        tab = value;
        viewport.scrollTop = 0;
    }

    function refreshHistory() {
        void history?.refresh();
    }

    async function repeat(entry: HistoryEntry, dismiss?: () => void) {
        const reveal = () => {
            dismiss?.();
            selectTab(entry.kind === "session" ? "session" : "ethernet");
        };
        return entry.kind === "session"
            ? await session.prepareRepeat(entry.report.interface, entry.target, reveal)
            : await ethernet.prepareRepeat(entry.report.link.interface, entry.target, reveal);
    }

    async function openTab(value: WorkspaceTab) {
        selectTab(value);
        await tick();
        document.getElementById(`${group}-tab-${value}`)?.focus();
    }

    async function runCommand(command: WorkspaceCommand, dismiss: () => void, signal: AbortSignal): Promise<ApiError | null> {
        if (command === "repeat-latest") {
            const result = await readHistory(signal);
            if (signal.aborted) return null;
            if (!result.ok) return result.error;
            const latest = result.entries[0];
            if (!latest) return { key: "history_empty", code: "ValidationError", severity: "info", args: [] };
            return await repeat(latest, dismiss)
                ? null
                : { key: "history_repeat_unavailable", code: "ValidationError", severity: "warning", args: [] };
        }

        dismiss();
        if (command === "appearance") {
            selectTab("settings");
            await settings.showAppearance();
        } else {
            await openTab(command);
        }
        return null;
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
        <CommandPalette {commands} shortcut={desktop} onRun={runCommand} />
    </div>
</header>

<main class="workspace-main" bind:this={viewport}>
    <div class="workspace-frame">
        <TabPanel {group} value="wifi" active={tab === "wifi"}>
            <Scanner />
            <RadioDiagnostics />
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
            <SettingsPanel bind:this={settings} onOpenHistory={() => openTab("history")} />
        </TabPanel>
    </div>
</main>
