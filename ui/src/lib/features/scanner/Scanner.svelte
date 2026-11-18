<script lang="ts">
    import { onMount } from "svelte";
    import { currentLang, t, translateError, type ApiError } from "../../i18n";
    import Alert from "../../components/ui/Alert.svelte";
    import SignalBadge from "../../components/ui/SignalBadge.svelte";
    import PointDetails from "./PointDetails.svelte";
    import ScanTable from "./ScanTable.svelte";
    import { fetchScan } from "./scan-api";
    import { bandOf, type AccessPoint } from "./scan-model";

    let points = $state.raw<AccessPoint[]>([]);
    let loading = $state(false);
    let error = $state<ApiError | null>(null);
    let updatedAt = $state<Date | null>(null);
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
            <p class="eyebrow">{t("scan_eyebrow")}</p>
            <h1 id="scan-heading">{t("scan_overview")}</h1>
            <p class="intro">{t("scan_intro")}</p>
        </div>
        <button type="button" class="btn" onclick={scan} disabled={loading}>
            <svg width="16" height="16" viewBox="0 0 20 20" fill="none" aria-hidden="true">
                <path d="M16 8a6 6 0 1 0-1.1 6.8M16 4v4h-4" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" />
            </svg>
            {loading ? t(updatedAt ? "scan_refreshing" : "scanning_btn") : t("scan_btn")}
        </button>
    </header>

    <p class="stamp" role="status">
        {#if updatedAt}
            {t("scan_updated_at")} <time datetime={updatedAt.toISOString()}>{dateFormat.format(updatedAt)}</time>
        {:else if loading}
            {t("scanning_air")}
        {/if}
    </p>

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
                {#each connected as point (point)}
                    <div class="connection-point">
                        <div class="connection-summary">
                            <div>
                                <p class="ssid">{point.ssid || t("scan_hidden_ssid")}</p>
                                <p class="muted">{t(`band_${bandOf(point.frequencyMhz)}`)} · {t("scan_mhz_value", [String(point.frequencyMhz)])}</p>
                            </div>
                            <div class="quality">
                                <span class="muted">{t("scan_quality")}</span>
                                <SignalBadge value={point.qualityPercent} missingLabel={t("scan_metric_missing")} />
                            </div>
                        </div>
                        <details>
                            <summary>{t("scan_details")}</summary>
                            <div class="details-body"><PointDetails {point} /></div>
                        </details>
                    </div>
                {/each}
            {/if}
        </section>

        <ScanTable {points} />
    {:else if !error}
        <div class="card pending" aria-hidden="true"><span></span><span></span><span></span></div>
    {/if}
</section>

<style>
    .overview { display: grid; gap: var(--space-3); min-width: 0; }
    .page-head { display: flex; align-items: flex-end; justify-content: space-between; gap: var(--space-3); }
    .eyebrow { margin: 0 0 var(--space-2); color: var(--color-primary); letter-spacing: .12em; text-transform: uppercase; font-size: .7rem; font-weight: 700; }
    h1 { margin: 0; font-size: clamp(1.7rem, 5vw, 2.6rem); letter-spacing: -.035em; line-height: 1.12; }
    .intro { color: var(--color-text-muted); margin: var(--space-2) 0 0; font-size: .925rem; }
    .btn { display: inline-flex; align-items: center; justify-content: center; gap: var(--space-2); white-space: nowrap; }
    .stamp { margin: 0; min-height: 1.2em; color: var(--color-text-muted); font-size: .8rem; }
    .stamp:empty { display: none; }
    h2 { margin: 0; font-size: .85rem; color: var(--color-text-muted); font-weight: 600; }
    .connection-card { display: grid; gap: var(--space-3); border-inline-start: 3px solid var(--color-primary); }
    .connection-point + .connection-point { border-top: 1px solid var(--color-border); padding-top: var(--space-3); }
    .connection-summary { display: flex; align-items: flex-start; justify-content: space-between; flex-wrap: wrap; gap: var(--space-3); }
    .connection-summary > div { min-width: 0; }
    .ssid { margin: 0 0 var(--space-1); font-size: 1.35rem; font-weight: 700; overflow-wrap: anywhere; }
    .muted, .previous { margin: 0; color: var(--color-text-muted); font-size: .85rem; }
    .previous { display: block; margin-top: var(--space-1); }
    .quality { display: grid; gap: var(--space-1); }
    details { margin-top: var(--space-3); }
    summary { cursor: pointer; color: var(--color-primary); font-size: .85rem; }
    .details-body { margin-top: var(--space-3); }
    .pending { display: grid; gap: var(--space-3); }
    .pending span { height: .8rem; background: var(--color-surface-raised); border-radius: var(--radius-md); }
    .pending span:nth-child(1) { width: 42%; }
    .pending span:nth-child(2) { width: 70%; }
    .pending span:nth-child(3) { width: 56%; }
    @media (max-width: 560px) {
        .page-head { align-items: flex-start; flex-direction: column; }
    }
</style>
