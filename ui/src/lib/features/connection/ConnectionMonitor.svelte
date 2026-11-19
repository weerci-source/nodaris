<script lang="ts">
    import ConnectionResult from "./ConnectionResult.svelte";
    import { onDestroy } from "svelte";
    import Alert from "../../components/ui/Alert.svelte";
    import Button from "../../components/ui/Button.svelte";
    import LoadingStatus from "../../components/ui/LoadingStatus.svelte";
    import { t, translateError, type ApiError } from "../../i18n";
    import { collectConnection, type ConnectionReport } from "./connection-api";

    let { wifiInterface }: { wifiInterface: string } = $props();

    const id = $props.id();
    let report = $state.raw<ConnectionReport | null>(null);
    let error = $state<ApiError | null>(null);
    let loading = $state(false);
    let cancelled = $state(false);
    let active: AbortController | null = null;

    async function run() {
        if (active) return;
        const request = new AbortController();
        active = request;
        error = null;
        cancelled = false;
        loading = true;
        const result = await collectConnection(wifiInterface, request.signal);
        if (request.signal.aborted) return;

        active = null;
        loading = false;
        if (result.ok) report = result.report;
        else error = result.error;
    }

    function cancel() {
        active?.abort();
        active = null;
        loading = false;
        cancelled = true;
    }

    onDestroy(() => active?.abort());
</script>

<section class="ui-stack ui-subsection" aria-labelledby={`${id}-heading`}>
    <header class="ui-row ui-row-between">
        <div>
            <h3 class="ui-title" id={`${id}-heading`}>{t("connection_title")}</h3>
            <p class="ui-muted">{t("connection_intro")}</p>
        </div>
        <div class="ui-row">
            <Button {loading} onclick={run}>{t("connection_start")}</Button>
            {#if loading}
                <Button variant="secondary" onclick={cancel}>{t("connection_cancel")}</Button>
            {/if}
        </div>
    </header>

    <LoadingStatus message={loading ? t(report ? "connection_refreshing" : "connection_loading") : ""} />
    {#if cancelled}<p class="ui-muted" role="status">{t("connection_cancelled")}</p>{/if}
    {#if error}<Alert severity={error.severity}>{translateError(error)}</Alert>{/if}

    {#if report}<ConnectionResult {report} />{/if}
</section>
