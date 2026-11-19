<script lang="ts">
    import { onMount } from "svelte";
    import { SvelteSet } from "svelte/reactivity";
    import { currentLang, t, translateError, type ApiError } from "../../i18n";
    import Button from "../../components/ui/Button.svelte";
    import Icon from "../../components/ui/Icon.svelte";
    import LoadingStatus from "../../components/ui/LoadingStatus.svelte";
    import Alert from "../../components/ui/Alert.svelte";
    import SignalBadge from "../../components/ui/SignalBadge.svelte";
    import PointDetails from "./PointDetails.svelte";
    import ScanTable from "./ScanTable.svelte";
    import { fetchScan } from "./scan-api";
    import { bandOf, type AccessPoint } from "./scan-model";

    let points = $state.raw<AccessPoint[]>([]);
    let loading = $state(true);
    let error = $state<ApiError | null>(null);
    let updatedAt = $state<Date | null>(null);
    const expanded = new SvelteSet<AccessPoint>();
    let connected = $derived(points.filter((point) => point.isConnected));
    const dateFormat = new Intl.DateTimeFormat(currentLang, { dateStyle: "short", timeStyle: "medium" });
    let active: AbortController | null = null;

    async function scan() {
        if (active) return;
        const controller = new AbortController();
        active = controller;
        loading = true;
        error = null;
        try {
            const result = await fetchScan(controller.signal);
            if (controller.signal.aborted) return;
            if (result.ok) {
                expanded.clear();
                points = result.points;
                updatedAt = new Date();
            } else {
                error = result.error;
            }
        } catch {
            if (!controller.signal.aborted) {
                error = { key: "scan_unknown_failure", code: "InternalError", severity: "error", args: [] };
            }
        } finally {
            active = null;
            if (!controller.signal.aborted) loading = false;
        }
    }

    onMount(() => {
        void scan();
        return () => active?.abort();
    });
</script>

<section class="overview" aria-labelledby="scan-heading">
    <header class="page-head">
        <div>
            <h1 id="scan-heading">{t("scan_overview")}</h1>
            <p class="intro">{t("scan_intro")}</p>
        </div>
        <Button icon="refresh" {loading} onclick={scan}>
            {t("scan_btn")}
        </Button>
    </header>

    <LoadingStatus message={loading ? t(updatedAt ? "scan_refreshing" : "scanning_air") : ""} />

    {#if updatedAt}
        <p class="stamp" role="status">
            {t("scan_updated_at")} <time datetime={updatedAt.toISOString()}>{dateFormat.format(updatedAt)}</time>
        </p>
    {/if}

    {#if error}
        <Alert severity={error.severity}>
            {translateError(error)}
            {#if updatedAt}<span class="previous">{t("scan_previous_results")}</span>{/if}
        </Alert>
    {/if}

    {#if updatedAt}
        <section class="card connection-card" aria-labelledby="connection-heading">
            <header><h2 id="connection-heading">{t("scan_current_connection")}</h2></header>
            {#if connected.length === 0}
                <p class="muted">{t("scan_connected_unknown")}</p>
            {:else}
                {#each connected as point, index (point)}
                    {@const isExpanded = expanded.has(point)}
                    <div class="connection-point">
                        <div class="connection-summary">
                            <div>
                                <p class="ssid">{point.ssid || t("scan_hidden_ssid")}</p>
                                <p class="connected"><Icon name="check" /> {t("scan_connected")}</p>
                                <p class="muted">{t(`band_${bandOf(point.frequencyMhz)}`)} · {t("scan_mhz_value", [String(point.frequencyMhz)])}</p>
                            </div>
                            <div class="quality">
                                <span class="muted">{t("scan_quality")}</span>
                                <SignalBadge value={point.qualityPercent} missingLabel={t("scan_metric_missing")} />
                            </div>
                        </div>
                        <div class="details">
                            <Button
                                variant="plain"
                                size="sm"
                                icon={isExpanded ? "chevron-down" : "chevron-right"}
                                aria-expanded={isExpanded}
                                aria-controls={`connection-details-${index}`}
                                onclick={() => {
                                    if (!expanded.delete(point)) expanded.add(point);
                                }}
                            >
                                {t("scan_details")}
                            </Button>
                            <div id={`connection-details-${index}`} class="details-body" hidden={!isExpanded}>
                                {#if isExpanded}<PointDetails {point} />{/if}
                            </div>
                        </div>
                    </div>
                {/each}
            {/if}
        </section>
    {/if}

    {#if updatedAt || loading}
        <ScanTable {points} loading={loading && !updatedAt} />
    {/if}
</section>

<style>
    .overview { display: grid; gap: var(--space-3); min-width: 0; }
    .page-head { display: flex; align-items: flex-end; justify-content: space-between; gap: var(--space-3); }
    h1 { margin: 0; font-size: clamp(1.7rem, 5vw, 2.6rem); letter-spacing: -.035em; line-height: 1.12; }
    .intro { color: var(--color-text-muted); margin: var(--space-2) 0 0; font-size: .925rem; }
    .stamp { margin: 0; min-height: 1.2em; color: var(--color-text-muted); font-size: .8rem; }
    h2 { margin: 0; font-size: .85rem; color: var(--color-text-muted); font-weight: 600; }
    .connection-card { display: grid; gap: var(--space-3); border-inline-start: var(--space-1) solid var(--color-primary); }
    .connection-point + .connection-point { border-top: 1px solid var(--color-border); padding-top: var(--space-3); }
    .connection-summary { display: flex; align-items: flex-start; justify-content: space-between; flex-wrap: wrap; gap: var(--space-3); }
    .connection-summary > div { min-width: 0; }
    .ssid { margin: 0 0 var(--space-1); font-size: 1.35rem; font-weight: 700; overflow-wrap: anywhere; }
    .muted, .previous { margin: 0; color: var(--color-text-muted); font-size: .85rem; }
    .previous { display: block; margin-top: var(--space-1); }
    .quality { display: grid; gap: var(--space-1); }
    .connected { display: flex; align-items: center; gap: var(--space-1); margin: 0 0 var(--space-1); color: var(--color-success); font-size: .85rem; }
    .details { margin-top: var(--space-2); color: var(--color-primary); }
    .details-body { margin-top: var(--space-3); color: var(--color-text); }
    @media (max-width: 560px) {
        .page-head { align-items: flex-start; flex-direction: column; }
    }
</style>
