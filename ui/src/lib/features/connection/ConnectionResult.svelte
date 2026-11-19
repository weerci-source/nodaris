<script lang="ts">
    import { currentLang, t } from "../../i18n";
    import type { ConnectionEndpoint, ConnectionReport } from "./connection-api";

    let { report }: { report: ConnectionReport } = $props();
    const formatTime = new Intl.DateTimeFormat(currentLang, { hour: "numeric", minute: "numeric", second: "numeric" });
    const formatSeconds = new Intl.NumberFormat(currentLang, { maximumFractionDigits: 1 });

    function stickyText(assessment: ConnectionReport["sticky_client"]["assessment"]) {
        if (assessment === "NoEvidence") return t("connection_sticky_no_evidence");
        if ("Suspected" in assessment) return t("connection_sticky_suspected");
        return t("connection_sticky_insufficient");
    }
</script>

{#snippet endpoint(sample: ConnectionEndpoint, title: string)}
    <div>
        <dt>{title}</dt>
        <dd>
            {sample.bssid ?? t("connection_no_link")}
            {#if sample.frequency_mhz !== null} · {t("radio_ui_mhz", [String(sample.frequency_mhz)])}{/if}
            {#if sample.rssi_dbm !== null} · {sample.rssi_dbm} {t("unit_dbm")}{/if}
        </dd>
        {#if sample.observed_at.unix_time_ms !== null}
            <dd class="ui-muted">
                <time datetime={new Date(sample.observed_at.unix_time_ms).toISOString()}>
                    {formatTime.format(sample.observed_at.unix_time_ms)}
                </time>
            </dd>
        {/if}
    </div>
{/snippet}

<p class="ui-muted" role="status">{t("connection_samples", [
    String(report.sample_count - report.unavailable_count),
    String(report.sample_count),
    formatSeconds.format(report.elapsed_ms / 1e3),
])}</p>
{#if report.transitions.length}
    <ol class="ui-stack connection-events">
        {#each report.transitions as transition}
            <li class="card ui-stack">
                <h4 class="ui-title">{t(`connection_${transition.kind}`)}</h4>
                <dl class="ui-metrics">
                    {@render endpoint(transition.before, t("connection_before"))}
                    {@render endpoint(transition.after, t("connection_after"))}
                </dl>
                <p class="ui-muted">{t(`connection_${transition.detection_method}`, [
                    transition.source === "linux.iw" ? "iw link" : transition.source,
                ])}</p>
            </li>
        {/each}
    </ol>
{:else}
    <p class="ui-muted">{t(report.sample_count - report.unavailable_count < 2
        ? "connection_missing_samples"
        : "connection_empty")}</p>
{/if}
<p class="ui-muted">{stickyText(report.sticky_client.assessment)}</p>
