<script lang="ts">
    import { currentLang, t } from "../../i18n";
    import type { EthernetLink } from "./ethernet-api";

    let { link }: { link: EthernetLink } = $props();
    let rate = $derived(new Intl.NumberFormat(currentLang()));
    let time = $derived(new Intl.DateTimeFormat(currentLang(), { dateStyle: "short", timeStyle: "medium" }));

    function unavailable(reason: string) {
        const key = `ethernet_reason_${reason}`;
        const message = t(key);
        return message === key ? t("scan_metric_missing") : message;
    }
</script>

<dl class="ui-metrics">
    <div>
        <dt>{t("ethernet_local_link")}</dt>
        <dd>{link.state.state.status === "available"
            ? t(`ethernet_link_${link.state.state.value}`) : unavailable(link.state.state.value)}</dd>
    </div>
    <div>
        <dt>{t("ethernet_speed")}</dt>
        <dd>{link.speed_mbps.state.status === "available"
            ? t("ethernet_speed_value", [rate.format(link.speed_mbps.state.value)])
            : unavailable(link.speed_mbps.state.value)}</dd>
    </div>
    {#if link.state.observed_at.unix_time_ms !== null}
        <div>
            <dt>{t("ethernet_link_observed_at")}</dt>
            <dd>{time.format(link.state.observed_at.unix_time_ms)}</dd>
        </div>
    {/if}
</dl>
