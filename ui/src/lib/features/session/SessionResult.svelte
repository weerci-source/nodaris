<script lang="ts">
    import { currentLang, t } from "../../i18n";
    import ConnectionResult from "../connection/ConnectionResult.svelte";
    import ProbeResult from "../probes/ProbeResult.svelte";
    import RadioResult from "../radio/RadioResult.svelte";
    import type { DiagnosticSession } from "./session-api";

    let { report }: { report: DiagnosticSession } = $props();
    const id = $props.id();
    let time = $derived(new Intl.DateTimeFormat(currentLang(), { dateStyle: "short", timeStyle: "medium" }));
    let number = $derived(new Intl.NumberFormat(currentLang(), { maximumFractionDigits: 2 }));

    function formatTime(value: number | null) {
        return value === null ? t("scan_metric_missing") : time.format(value);
    }

    function sectionReason(status: "unavailable" | "not_collected", reason?: string) {
        const key = `session_status_${reason ?? status}`;
        const message = t(key);
        return message === key ? t("session_status_unavailable") : message;
    }
</script>

<p class="ui-muted">{t("session_window", [
    report.interface, formatTime(report.started_at.unix_time_ms), formatTime(report.finished_at.unix_time_ms), String(report.rules_version),
])}</p>

<section class="ui-stack ui-subsection" aria-labelledby={`${id}-findings`}>
    <h3 class="ui-title" id={`${id}-findings`}>{t("session_findings")}</h3>
    {#each report.hypotheses as finding (finding.key)}
        <article class="card ui-stack">
            <h4 class="ui-title">{t(`session_finding_${finding.key}`)}</h4>
            {#if finding.key === "beyond_gateway"}
                <p class="ui-muted">{t("session_beyond_evidence", [
                    number.format(finding.gateway_avg_rtt_ms), number.format(finding.external_avg_rtt_ms),
                    formatTime(finding.started_at_ms), formatTime(finding.finished_at_ms),
                ])}</p>
                <p class="ui-muted">{t("session_beyond_limit")}</p>
            {:else}
                <p class="ui-muted">{t("session_sticky_evidence", [
                    finding.evidence.candidate_bssid, number.format(finding.evidence.minimum_signal_advantage_db),
                    number.format(finding.evidence.duration_ns / 1e9),
                ])}</p>
                <p class="ui-muted">{t("session_sticky_limit")}</p>
            {/if}
        </article>
    {:else}
        <p class="ui-muted">{t("session_no_findings")}</p>
    {/each}
</section>

<section class="ui-stack ui-subsection" aria-labelledby={`${id}-radio`}>
    <h3 class="ui-title" id={`${id}-radio`}>{t("radio_ui_title")}</h3>
    {#if report.radio.status === "available"}
        <RadioResult radio={report.radio.data} />
    {:else}
        <p class="ui-muted">{sectionReason(report.radio.status, report.radio.status === "unavailable" ? report.radio.reason : undefined)}</p>
    {/if}
</section>

<section class="ui-stack ui-subsection" aria-labelledby={`${id}-probes`}>
    <h3 class="ui-title" id={`${id}-probes`}>{t("probe_title")}</h3>
    {#if report.probes.status === "available"}
        <ProbeResult report={report.probes.data} />
    {:else}
        <p class="ui-muted">{sectionReason(report.probes.status, report.probes.status === "unavailable" ? report.probes.reason : undefined)}</p>
    {/if}
</section>

<section class="ui-stack ui-subsection" aria-labelledby={`${id}-connection`}>
    <h3 class="ui-title" id={`${id}-connection`}>{t("connection_title")}</h3>
    {#if report.connection.status === "available"}
        <ConnectionResult report={report.connection.data} />
    {:else}
        <p class="ui-muted">{sectionReason(report.connection.status, report.connection.status === "unavailable" ? report.connection.reason : undefined)}</p>
    {/if}
</section>
