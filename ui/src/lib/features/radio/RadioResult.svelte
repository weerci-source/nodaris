<script lang="ts">
    import Icon from "../../components/ui/Icon.svelte";
    import SignalBadge from "../../components/ui/SignalBadge.svelte";
    import { currentLang, t } from "../../i18n";
    import type { Radio } from "./radio-api";

    let { radio }: { radio: Radio } = $props();
    const id = $props.id();
    let formatMetric = $derived(new Intl.NumberFormat(currentLang(), { maximumFractionDigits: 2 }));

    function reasonText(reason: string) {
        switch (reason) {
            case "hardware_unavailable": return t("radio_ui_hardware_unavailable");
            case "rssi_unavailable": return t("radio_ui_rssi_unavailable");
            case "noise_unavailable": return t("radio_ui_noise_unavailable");
            case "different_sessions": return t("radio_ui_different_sessions");
            case "time_mismatch": return t("radio_ui_time_mismatch");
            case "missing_frequency": return t("radio_ui_missing_frequency");
            case "permission_denied": return t("radio_ui_permission_denied");
            default: return t("radio_ui_unavailable");
        }
    }

    function retryReasonText(reason: string) {
        const key = `radio_ui_tx_${reason}`;
        const message = t(key);
        return message === key ? reasonText(reason) : message;
    }

    function surveyReasonText(reason: string) {
        const key = `radio_ui_survey_${reason}`;
        const message = t(key);
        return message === key ? reasonText(reason) : message;
    }

    function sourceText(source: string) {
        const label = source === "linux.iw.survey" ? "iw survey dump"
            : source === "linux.iw" ? "iw link" : source;
        return t("radio_ui_source", [label]);
    }
</script>

{#if radio.status === "unavailable"}
    <p class="ui-muted" role="status"><Icon name="warning" /> {reasonText(radio.reason)}</p>
{:else if radio.status === "available"}
    <dl class="ui-metrics" aria-live="polite">
        <div>
            <dt>{t("radio_ui_frequency")}</dt>
            <dd>{radio.frequency_mhz === null ? t("radio_ui_unavailable") : t("radio_ui_mhz", [String(radio.frequency_mhz)])}</dd>
        </div>
        <div>
            <dt>{t("radio_ui_quality")}</dt>
            <dd><SignalBadge value={radio.signal_quality_percent} missingLabel={t("radio_ui_unavailable")} /></dd>
        </div>
        <div>
            <dt>RSSI</dt>
            <dd>{radio.rssi_dbm == null ? t("radio_ui_unavailable") : `${radio.rssi_dbm} ${t("unit_dbm")}`}</dd>
            {#if radio.rssi_source}<dd class="ui-muted"><small>{sourceText(radio.rssi_source)}</small></dd>{/if}
        </div>
        {#if radio.noise}
            <div>
                <dt>{t("radio_ui_noise_floor")}</dt>
                {#if radio.noise.status === "available"}
                    <dd>{radio.noise.dbm} {t("unit_dbm")}</dd>
                    <dd class="ui-muted"><small>{sourceText(radio.noise.source)}</small></dd>
                {:else}
                    <dd>{surveyReasonText(radio.noise.reason)}</dd>
                {/if}
            </div>
        {/if}
        <div>
            <dt>SNR</dt>
            <dd>{radio.snr.status === "available" ? `${radio.snr.db} ${t("unit_db")}` : reasonText(radio.snr.reason)}</dd>
        </div>
    </dl>
    {#if radio.airtime && radio.airtime.status !== "not_collected"}
        <section class="ui-stack ui-subsection" aria-labelledby={`${id}-airtime`}>
            <h3 class="ui-title" id={`${id}-airtime`}>{t("radio_ui_airtime_title")}</h3>
            {#if radio.airtime.status === "available"}
                <dl class="ui-metrics">
                    <div>
                        <dt>{t("radio_ui_airtime_busy_share")}</dt>
                        <dd>{formatMetric.format(radio.airtime.utilization_percent)}%</dd>
                    </div>
                    <div>
                        <dt>{t("radio_ui_airtime_busy_time")}</dt>
                        <dd>{t("radio_ui_ms_value", [formatMetric.format(radio.airtime.busy_delta_us / 1e3)])}</dd>
                    </div>
                    <div>
                        <dt>{t("radio_ui_airtime_total_time")}</dt>
                        <dd>{t("radio_ui_ms_value", [formatMetric.format(radio.airtime.total_delta_us / 1e3)])}</dd>
                    </div>
                    <div>
                        <dt>{t("radio_ui_tx_interval")}</dt>
                        <dd>{t("radio_ui_seconds_value", [formatMetric.format(radio.airtime.interval_ns / 1e9)])}</dd>
                    </div>
                </dl>
            {:else}
                <p class="ui-muted" role="status">
                    <Icon name="warning" /> {surveyReasonText(radio.airtime.reason)}
                </p>
            {/if}
        </section>
    {/if}
    {#if radio.retries && radio.retries.status !== "not_collected"}
        <section class="ui-stack ui-subsection" aria-labelledby={`${id}-tx`}>
            <h3 class="ui-title" id={`${id}-tx`}>{t("radio_ui_tx_title")}</h3>
            {#if radio.retries.status === "available"}
                <dl class="ui-metrics">
                    <div>
                        <dt>{t("radio_ui_tx_retries")}</dt>
                        <dd>{radio.retries.retries_delta ?? t("radio_ui_unavailable")}</dd>
                    </div>
                    <div>
                        <dt>{t("radio_ui_tx_failures")}</dt>
                        <dd>{radio.retries.failed_delta ?? t("radio_ui_unavailable")}</dd>
                    </div>
                    <div>
                        <dt>{t("radio_ui_tx_interval")}</dt>
                        <dd>{t("radio_ui_seconds_value", [formatMetric.format(radio.retries.interval_ns / 1e9)])}</dd>
                    </div>
                    {#if radio.retries.retry_rate_percent !== null}
                        <div>
                            <dt>{t("radio_ui_tx_rate")}</dt>
                            <dd>{radio.retries.retry_rate_percent}%</dd>
                        </div>
                    {/if}
                </dl>
                {#if radio.retries.attempts_delta === null}
                    <p class="ui-muted">{t("radio_ui_tx_missing_attempt_counter")}</p>
                {/if}
            {:else}
                <p class="ui-muted" role="status">
                    <Icon name="warning" /> {retryReasonText(radio.retries.reason)}
                </p>
            {/if}
        </section>
    {/if}
{/if}
