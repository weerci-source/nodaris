<script lang="ts">
    import { t } from "../../i18n";
    import type { AccessPoint } from "./scan-model";

    let { point }: { point: AccessPoint } = $props();
    const missing = () => t("scan_metric_missing");
</script>

<dl>
    <div><dt>{t("table_bssid")}</dt><dd class="mono">{point.bssid || missing()}</dd></div>
    <div><dt>{t("scan_frequency")}</dt><dd>{t("scan_mhz_value", [String(point.frequencyMhz)])}</dd></div>
    <div><dt>{t("scan_channel")}</dt><dd>{point.channel ?? missing()}</dd></div>
    <div><dt>{t("scan_width")}</dt><dd>{point.channelWidthMhz === null ? missing() : t("scan_mhz_value", [String(point.channelWidthMhz)])}</dd></div>
    <div><dt>RSSI</dt><dd>{point.rssiDbm === null ? missing() : t("scan_dbm_value", [String(point.rssiDbm)])}</dd></div>
</dl>

<style>
    dl { margin: 0; display: grid; grid-template-columns: repeat(auto-fit, minmax(min(100%, 8rem), 1fr)); gap: var(--space-3); }
    dt { color: var(--color-text-muted); font-size: .75rem; margin-bottom: var(--space-1); }
    dd { margin: 0; font-size: .875rem; overflow-wrap: anywhere; }
    .mono { font-family: ui-monospace, monospace; }
</style>
