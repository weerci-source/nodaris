<script lang="ts">
    import { onMount, tick } from "svelte";
    import Alert from "../../components/ui/Alert.svelte";
    import Button from "../../components/ui/Button.svelte";
    import Icon from "../../components/ui/Icon.svelte";
    import LoadingStatus from "../../components/ui/LoadingStatus.svelte";
    import Skeleton from "../../components/ui/Skeleton.svelte";
    import { currentLang, t, translateError, type ApiError } from "../../i18n";
    import EthernetLinkResult from "../ethernet/EthernetLinkResult.svelte";
    import ProbeResult from "../probes/ProbeResult.svelte";
    import SessionResult from "../session/SessionResult.svelte";
    import { readHistory, type HistoryEntry } from "./history-api";

    let { onRepeat }: { onRepeat: (entry: HistoryEntry) => Promise<boolean> } = $props();
    const id = $props.id();
    let time = $derived(new Intl.DateTimeFormat(currentLang(), { dateStyle: "short", timeStyle: "medium" }));
    let entries = $state.raw<HistoryEntry[] | null>(null);
    let selectedId = $state("");
    let selected = $derived(entries?.find((entry) => entry.id === selectedId));
    const narrow = window.matchMedia("(max-width: 760px)");
    let compact = $state(narrow.matches);
    let detailOpen = $state(false);
    let actionsOpen = $state(false);
    let confirming = $state(false);
    let loading = $state(true);
    let deleting = $state(false);
    let loadError = $state<ApiError | null>(null);
    let actionError = $state<ApiError | null>(null);
    let deleted = $state(false);
    let heading: HTMLHeadingElement;
    let detail = $state<HTMLElement>();
    let actions = $state<HTMLDivElement>();
    let active: AbortController | null = null;
    let refreshPending = false;

    function applyEntries(next: HistoryEntry[]) {
        entries = next;
        if (!next.some((entry) => entry.id === selectedId)) {
            selectedId = next[0]?.id ?? "";
            confirming = false;
            actionsOpen = false;
            actionError = null;
            detailOpen = false;
        }
    }

    function focus(part: string) {
        document.getElementById(`${id}-${part}`)?.focus();
    }

    function selectReport(entry: HistoryEntry) {
        selectedId = entry.id;
        detailOpen = true;
        actionsOpen = false;
        confirming = false;
        actionError = null;
        deleted = false;
        if (compact) void tick().then(() => {
            detail?.scrollIntoView({ block: "start" });
            focus("report-title");
        });
    }

    function back() {
        detailOpen = false;
        confirming = false;
        actionsOpen = false;
        actionError = null;
        void tick().then(() => focus(`entry-${selectedId}`));
    }

    function handleActions(event: MouseEvent) {
        if (actionsOpen && event.target instanceof Node && !actions?.contains(event.target)) {
            actionsOpen = false;
        }
    }

    function handleEscape(event: KeyboardEvent) {
        if (event.key !== "Escape" || deleting || !detail?.contains(document.activeElement)) return;
        if (actionsOpen || confirming) {
            cancelAction();
        } else if (compact && detailOpen) {
            back();
        } else {
            return;
        }
        event.preventDefault();
    }

    function toggleActions() {
        actionsOpen = !actionsOpen;
        if (actionsOpen) void tick().then(() => focus("delete"));
    }

    function confirmDelete() {
        actionsOpen = false;
        confirming = true;
        actionError = null;
        deleted = false;
        void tick().then(() => focus("confirm"));
    }

    function cancelAction() {
        actionsOpen = false;
        confirming = false;
        void tick().then(() => focus("actions"));
    }

    export async function refresh() {
        if (deleting) {
            refreshPending = true;
            return;
        }
        active?.abort();
        const controller = new AbortController();
        active = controller;
        loading = true;
        loadError = null;
        const result = await readHistory(controller.signal);
        if (controller.signal.aborted) return;
        active = null;
        loading = false;
        if (result.ok) applyEntries(result.entries);
        else loadError = result.error;
    }

    async function remove() {
        if (deleting || !selected) return;
        actionError = null;
        deleted = false;
        active?.abort();
        const controller = new AbortController();
        active = controller;
        deleting = true;
        loading = false;
        const result = await readHistory(controller.signal, selected.id);
        if (controller.signal.aborted) return;
        active = null;
        deleting = false;
        if (result.ok) {
            applyEntries(result.entries);
            loadError = null;
            confirming = false;
            deleted = true;
        } else {
            actionError = result.error;
        }
        if (refreshPending) {
            refreshPending = false;
            void refresh();
        }
        await tick();
        if (!result.ok) focus("confirm");
        else if (selectedId) focus(`entry-${selectedId}`);
        else heading.focus();
    }

    async function repeat() {
        if (!selected) return;
        actionError = null;
        confirming = false;
        if (!await onRepeat(selected)) {
            actionError = { key: "history_repeat_unavailable", code: "ValidationError", severity: "warning", args: [] };
        }
    }

    function reportTime(entry: HistoryEntry) {
        const timestamp = entry.kind === "session"
            ? entry.report.started_at.unix_time_ms : entry.report.probes.started_at_unix_ms;
        return timestamp === null ? t("scan_metric_missing") : time.format(timestamp);
    }

    function reportInterface(entry: HistoryEntry) {
        const name = entry.kind === "session" ? entry.report.interface : entry.report.link.interface;
        return entry.target ? `${name} · ${entry.target}` : name;
    }

    onMount(() => {
        const update = () => { compact = narrow.matches; };
        narrow.addEventListener("change", update);
        void refresh();
        return () => {
            active?.abort();
            narrow.removeEventListener("change", update);
        };
    });
</script>

<svelte:window onclick={handleActions} onkeydown={handleEscape} />

{#snippet showError(error: ApiError)}
    <Alert severity={error.severity}>
        {translateError(error)}
        {#if error.request_id}
            <p class="ui-muted">{t("session_request_id", [error.request_id])}</p>
        {/if}
    </Alert>
{/snippet}

<section id="diagnostic-history" class="ui-stack ui-section" aria-labelledby={`${id}-title`}>
    <header class="ui-row ui-row-between">
        <h2 class="workspace-title" id={`${id}-title`} tabindex="-1" bind:this={heading}>{t("history_title")}</h2>
        <Button variant="secondary" loading={loading} disabled={deleting} onclick={refresh}>
            {t("history_refresh")}
        </Button>
    </header>

    {#if loading && entries === null}<div class="card"><Skeleton /></div>{/if}
    <LoadingStatus message={loading ? t(entries === null ? "history_loading" : "history_refreshing") : ""} />
    {#if loadError}{@render showError(loadError)}{/if}
    {#if deleted}<p class="ui-muted" role="status">{t("history_deleted")}</p>{/if}
    {#if entries?.length === 0}<p class="card ui-muted" role="status">{t("history_empty")}</p>{/if}

    {#if entries?.length && selected}
        <div class="history-layout" class:compact>
            <ul class="history-list" aria-label={t("history_reports")} hidden={compact && detailOpen}>
                {#each entries as entry (entry.id)}
                    <li>
                        <button type="button" class="history-entry" disabled={deleting}
                            id={`${id}-entry-${entry.id}`} aria-current={selectedId === entry.id ? "true" : undefined}
                            aria-controls={`${id}-report`} onclick={() => selectReport(entry)}>
                            <span class="history-entry-icon"><Icon name={entry.kind === "session" ? "session" : "ethernet"} /></span>
                            <span class="history-entry-text">
                                <strong>{reportInterface(entry)}</strong>
                                <span>{t(entry.kind === "session" ? "workspace_session" : "ethernet_title")} · {reportTime(entry)}</span>
                            </span>
                        </button>
                    </li>
                {/each}
            </ul>
            <article class="card ui-stack history-detail" id={`${id}-report`} aria-labelledby={`${id}-report-title`}
                bind:this={detail} hidden={compact && !detailOpen}>
                {#if compact}
                    <div><Button variant="plain" size="sm" icon="chevron-left" disabled={deleting} onclick={back}>{t("history_back")}</Button></div>
                {/if}
                <header class="ui-row ui-row-between">
                    <div>
                        <h3 class="ui-title" id={`${id}-report-title`} tabindex="-1">
                            {t(selected.kind === "session" ? "history_kind_session" : "ethernet_title")}
                        </h3>
                        <p class="ui-muted">{reportInterface(selected)} · {reportTime(selected)}</p>
                    </div>
                    <div class="ui-row history-controls">
                        <Button disabled={deleting || confirming} onclick={repeat}>{t("history_repeat")}</Button>
                        <div class="history-actions" bind:this={actions}>
                            <Button variant="secondary" icon="more" id={`${id}-actions`}
                                aria-label={t("history_actions")} aria-expanded={actionsOpen}
                                aria-controls={actionsOpen ? `${id}-action-list` : undefined}
                                disabled={deleting || confirming} onclick={toggleActions}>{""}</Button>
                            {#if actionsOpen}
                                <div class="history-action-list" id={`${id}-action-list`}>
                                    <Button variant="plain" class="history-danger" id={`${id}-delete`} onclick={confirmDelete}>
                                        {t("history_delete")}
                                    </Button>
                                </div>
                            {/if}
                        </div>
                    </div>
                </header>
                {#if confirming}
                    <div class="ui-stack ui-subsection">
                        <p id={`${id}-delete-confirmation`} class="ui-muted">{t("history_delete_prompt")}</p>
                        <div class="ui-row">
                            <Button variant="secondary" class="history-danger" loading={deleting}
                                id={`${id}-confirm`} aria-describedby={`${id}-delete-confirmation`} onclick={remove}>
                                {t("history_delete_confirm")}
                            </Button>
                            <Button variant="secondary" disabled={deleting} onclick={cancelAction}>{t("connection_cancel")}</Button>
                        </div>
                    </div>
                {/if}
                {#if actionError}{@render showError(actionError)}{/if}
                {#key selected.id}
                    {#if selected.kind === "session"}
                        <SessionResult report={selected.report} />
                    {:else}
                        <EthernetLinkResult link={selected.report.link} />
                        <section class="ui-stack ui-subsection" aria-labelledby={`${id}-probes`}>
                            <h3 class="ui-title" id={`${id}-probes`}>{t("probe_title")}</h3>
                            <ProbeResult report={selected.report.probes} />
                        </section>
                    {/if}
                {/key}
            </article>
        </div>
    {/if}
</section>
