<script lang="ts">
    import { onDestroy } from "svelte";
    import Alert from "../../components/ui/Alert.svelte";
    import Button from "../../components/ui/Button.svelte";
    import LoadingStatus from "../../components/ui/LoadingStatus.svelte";
    import { currentLang, t, translateError, type ApiError } from "../../i18n";
    import { collectProbes, probeError, type ProbeReport } from "./probe-api";

    let { wifiInterface }: { wifiInterface: string } = $props();

    const id = $props.id();
    const numberFormat = new Intl.NumberFormat(currentLang, { maximumFractionDigits: 3 });
    const timeFormat = new Intl.DateTimeFormat(currentLang, { dateStyle: "short", timeStyle: "medium" });
    let target = $state("");
    let report = $state.raw<ProbeReport | null>(null);
    let error = $state<ApiError | null>(null);
    let targetError = $state<ApiError | null>(null);
    let loading = $state(false);
    let active: AbortController | null = null;

    async function run(event: SubmitEvent) {
        event.preventDefault();
        if (active) return;
        error = null;
        targetError = null;
        const address = target.trim();
        if (!/^(?:0|[1-9]\d{0,2})(?:\.(?:0|[1-9]\d{0,2})){3}$/.test(address) ||
            address.split(".").some((octet) => Number(octet) > 255)) {
            targetError = probeError("diagnostics_probe_invalid_request", "ValidationError");
            return;
        }

        const controller = new AbortController();
        active = controller;
        loading = true;
        const result = await collectProbes(wifiInterface, address, controller.signal);
        if (controller.signal.aborted) return;
        active = null;
        loading = false;
        if (result.ok) report = result.report;
        else if (result.error.key === "diagnostics_probe_invalid_request") targetError = result.error;
        else error = result.error;
    }

    function milliseconds(value: number | null) {
        return value === null ? t("scan_metric_missing") : t("probe_ms_value", [numberFormat.format(value)]);
    }

    onDestroy(() => active?.abort());
</script>

<section class="ui-stack ui-subsection" aria-labelledby={`${id}-heading`}>
    <header>
        <h3 class="ui-title" id={`${id}-heading`}>{t("probe_title")}</h3>
        <p class="ui-muted">{t("probe_intro")}</p>
    </header>

    <form class="ui-row" onsubmit={run} novalidate>
        <label class="ui-field" for={`${id}-target`}>
            <span>{t("probe_target")}</span>
            <input
                id={`${id}-target`}
                class="ui-control"
                type="text"
                inputmode="decimal"
                maxlength="15"
                required
                disabled={loading}
                bind:value={target}
                aria-invalid={Boolean(targetError)}
                aria-describedby={targetError ? `${id}-error` : undefined}
                oninput={() => { targetError = null; }}
            />
        </label>
        <Button type="submit" {loading}>{t("probe_start")}</Button>
        {#if targetError}
            <div class="ui-field-error" id={`${id}-error`}>
                <Alert severity={targetError.severity}>{translateError(targetError)}</Alert>
            </div>
        {/if}
    </form>

    <LoadingStatus message={loading ? t(report ? "probe_refreshing" : "probe_collecting") : ""} />

    {#if error}
        <Alert severity={error.severity}>{translateError(error)}</Alert>
    {/if}

    {#if report}
        <p class="ui-muted" role="status">
            {t("probe_started_at", [timeFormat.format(report.started_at_unix_ms)])}
        </p>
        <div class="ui-metrics">
            {#each [report.gateway, report.external] as series, index}
                <article class="card ui-stack">
                    <h4 class="ui-title">{t(index === 0 ? "probe_gateway" : "probe_external")}</h4>
                    <dl class="ui-metrics">
                        {#if series.target}
                            <div><dt>{t("probe_destination")}</dt><dd>{series.target}</dd></div>
                        {/if}
                        {#if series.route}
                            <div>
                                <dt>{t("probe_route_interface")}</dt>
                                <dd>{series.route.interface}</dd>
                            </div>
                            {#if series.route.gateway}
                                <div><dt>{t("probe_route_gateway")}</dt><dd>{series.route.gateway}</dd></div>
                            {/if}
                        {/if}
                        <div>
                            <dt>{t("probe_duration")}</dt>
                            <dd>{milliseconds(series.finished_offset_ms - series.started_offset_ms)}</dd>
                        </div>
                        {#if series.statistics}
                            <div><dt>{t("probe_sent")}</dt><dd>{series.statistics.sent}</dd></div>
                            <div><dt>{t("probe_received")}</dt><dd>{series.statistics.received}</dd></div>
                            <div>
                                <dt>{t("probe_loss")}</dt>
                                <dd>{numberFormat.format(series.statistics.packet_loss_percent)}%</dd>
                            </div>
                            <div>
                                <dt>{t("probe_rtt")}</dt>
                                <dd>{[
                                    series.statistics.min_rtt_ms,
                                    series.statistics.avg_rtt_ms,
                                    series.statistics.max_rtt_ms,
                                ].map(milliseconds).join(" / ")}</dd>
                            </div>
                            <div><dt>{t("probe_jitter")}</dt><dd>{milliseconds(series.statistics.jitter_ms)}</dd></div>
                        {/if}
                    </dl>
                    {#if series.unavailable_reason}
                        {@const key = `probe_reason_${series.unavailable_reason}`}
                        <Alert severity="warning">{t(key) === key ? t("scan_metric_missing") : t(key)}</Alert>
                    {:else if series.route && series.route.interface !== report.interface}
                        <p class="ui-muted">{t("probe_other_interface")}</p>
                    {/if}
                </article>
            {/each}
        </div>
    {/if}
</section>
