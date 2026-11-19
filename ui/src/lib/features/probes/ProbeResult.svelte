<script lang="ts">
    import Button from "../../components/ui/Button.svelte";
    import Alert from "../../components/ui/Alert.svelte";
    import { currentLang, t } from "../../i18n";
    import type { ProbeReport } from "./probe-api";

    let { report }: { report: ProbeReport } = $props();
    const id = $props.id();
    let expanded = $state<number | null>(null);
    const numberFormat = new Intl.NumberFormat(currentLang, { maximumFractionDigits: 3 });
    const timeFormat = new Intl.DateTimeFormat(currentLang, { dateStyle: "short", timeStyle: "medium" });

    function milliseconds(value: number | null) {
        return value === null ? t("scan_metric_missing") : t("probe_ms_value", [numberFormat.format(value)]);
    }
</script>

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
            {#if series.attempts?.length}
                <Button variant="plain" size="sm" aria-expanded={expanded === index}
                    aria-controls={`${id}-attempts-${index}`}
                    icon={expanded === index ? "chevron-down" : "chevron-right"}
                    onclick={() => { expanded = expanded === index ? null : index; }}>
                    {t("probe_attempts")}
                </Button>
                <ol id={`${id}-attempts-${index}`} hidden={expanded !== index}>
                        {#each series.attempts as attempt}
                            <li>
                                {milliseconds(attempt.started_offset_ms)} ·
                                {attempt.outcome.status === "reply"
                                    ? t("probe_attempt_reply", [milliseconds(attempt.outcome.rtt_ms)])
                                    : t(`probe_attempt_${attempt.outcome.status}`)}
                            </li>
                        {/each}
                    </ol>
            {/if}
        </article>
    {/each}
</div>
