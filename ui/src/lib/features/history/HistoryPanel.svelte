<script lang="ts">
    import { onMount, tick } from "svelte";
    import Alert from "../../components/ui/Alert.svelte";
    import Button from "../../components/ui/Button.svelte";
    import LoadingStatus from "../../components/ui/LoadingStatus.svelte";
    import Skeleton from "../../components/ui/Skeleton.svelte";
    import { currentLang, t, translateError, type ApiError } from "../../i18n";
    import EthernetLinkResult from "../ethernet/EthernetLinkResult.svelte";
    import ProbeResult from "../probes/ProbeResult.svelte";
    import SessionResult from "../session/SessionResult.svelte";
    import { readHistory, type HistoryEntry } from "./history-api";

    let { onRepeat }: { onRepeat: (entry: HistoryEntry) => Promise<boolean> } = $props();
    const id = $props.id();
    const time = new Intl.DateTimeFormat(currentLang, { dateStyle: "short", timeStyle: "medium" });
    let entries = $state.raw<HistoryEntry[] | null>(null);
    let selectedId = $state("");
    let selected = $derived(entries?.find((entry) => entry.id === selectedId));
    let confirming = $state(false);
    let loading = $state(true);
    let deleting = $state(false);
    let loadError = $state<ApiError | null>(null);
    let actionError = $state<ApiError | null>(null);
    let status = $state("");
    let heading: HTMLHeadingElement;
    let selection = $state<HTMLSelectElement>();
    let active: AbortController | null = null;
    let refreshPending = false;

    function applyEntries(next: HistoryEntry[]) {
        entries = next;
        if (!next.some((entry) => entry.id === selectedId)) {
            selectedId = next[0]?.id ?? "";
            confirming = false;
        }
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
        status = "";
        if (!confirming) {
            confirming = true;
            return;
        }
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
            status = t("history_deleted");
        } else {
            actionError = result.error;
        }
        if (refreshPending) {
            refreshPending = false;
            void refresh();
        }
        await tick();
        (selection ?? heading)?.focus();
    }

    async function repeat() {
        if (!selected) return;
        actionError = null;
        confirming = false;
        if (!await onRepeat(selected)) {
            actionError = { key: "history_repeat_unavailable", code: "ValidationError", severity: "warning", args: [] };
        }
    }

    function entryLabel(entry: HistoryEntry) {
        const timestamp = entry.kind === "session"
            ? entry.report.started_at.unix_time_ms : entry.report.probes.started_at_unix_ms;
        const name = entry.kind === "session" ? entry.report.interface : entry.report.link.interface;
        const type = t(entry.kind === "session" ? "history_kind_session" : "ethernet_title");
        return [entry.target, timestamp === null ? t("scan_metric_missing") : time.format(timestamp), type, name]
            .filter(Boolean).join(" · ");
    }

    onMount(() => {
        void refresh();
        return () => active?.abort();
    });
</script>

{#snippet showError(error: ApiError)}
    <Alert severity={error.severity}>
        {translateError(error)}
        {#if error.request_id}
            <p class="ui-muted">{t("session_request_id", [error.request_id])}</p>
        {/if}
    </Alert>
{/snippet}

<section id="diagnostic-history" class="card ui-stack ui-section" aria-labelledby={`${id}-title`}>
    <header class="ui-row ui-row-between">
        <div>
            <h2 class="ui-title" id={`${id}-title`} tabindex="-1" bind:this={heading}>{t("history_title")}</h2>
            <p class="ui-muted">{t("history_retention")}</p>
        </div>
        <Button variant="secondary" loading={loading} disabled={deleting} onclick={refresh}>
            {t("history_refresh")}
        </Button>
    </header>

    {#if loading && entries === null}<Skeleton />{/if}
    <LoadingStatus message={loading ? t(entries === null ? "history_loading" : "history_refreshing") : ""} />
    {#if loadError}{@render showError(loadError)}{/if}
    {#if actionError}{@render showError(actionError)}{/if}
    {#if status}<p class="ui-muted" role="status">{status}</p>{/if}
    {#if entries?.length === 0}<p class="ui-muted" role="status">{t("history_empty")}</p>{/if}

    {#if entries?.length && selected}
        <div class="ui-row">
            <label class="ui-field">
                <span>{t("history_report")}</span>
                <select class="ui-control" bind:this={selection} bind:value={selectedId} disabled={deleting}
                    onchange={() => { confirming = false; actionError = null; status = ""; }}>
                    {#each entries as entry (entry.id)}
                        <option value={entry.id}>{entryLabel(entry)}</option>
                    {/each}
                </select>
            </label>
            <Button variant="secondary" disabled={deleting} onclick={repeat}>
                {t("history_repeat")}
            </Button>
            <Button variant="secondary" loading={deleting} onclick={remove}
                aria-describedby={confirming ? `${id}-delete-confirmation` : undefined}>
                {t(confirming ? "history_delete_confirm" : "history_delete")}
            </Button>
            {#if confirming}
                <Button variant="secondary" disabled={deleting} onclick={() => { confirming = false; }}>
                    {t("connection_cancel")}
                </Button>
            {/if}
        </div>
        {#if confirming}
            <p id={`${id}-delete-confirmation`} class="ui-muted" role="status">{t("history_delete_prompt")}</p>
        {/if}
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
    {/if}
</section>
