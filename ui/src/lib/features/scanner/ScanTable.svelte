<script lang="ts">
    import { currentLang, t } from "../../i18n";
    import SignalBadge from "../../components/ui/SignalBadge.svelte";
    import PointDetails from "./PointDetails.svelte";
    import { bandOf, selectPoints, type AccessPoint, type Band, type ScanSort } from "./scan-model";

    let { points }: { points: AccessPoint[] } = $props();
    const bands = ["all", "24g", "5g", "6g", "unknown"] as const;
    let query = $state("");
    let band = $state<Band | "all">("all");
    let sort = $state<ScanSort>("quality");
    let expanded = $state.raw<AccessPoint | null>(null);
    let visible = $derived(selectPoints(points, query, band, sort, currentLang));
    let filtered = $derived(query.trim() !== "" || band !== "all");

    function clearFilters() {
        query = "";
        band = "all";
        expanded = null;
    }
</script>

<section class="card table-card" aria-labelledby="nearby-heading">
    <header class="section-head">
        <div>
            <h2 id="nearby-heading">{t("scan_nearby")}</h2>
            <p aria-live="polite">{t("scan_results_count", [String(visible.length), String(points.length)])}</p>
        </div>
        {#if filtered}
            <button type="button" class="secondary" onclick={clearFilters}>{t("scan_clear_filters")}</button>
        {/if}
    </header>
    {#if points.length > 0}
        <div class="tool-row">
            <label class="search">
                <span>{t("scan_search")}</span>
                <input type="search" bind:value={query} placeholder={t("scan_search_placeholder")} />
            </label>
            <label>
                <span>{t("table_band")}</span>
                <select bind:value={band}>
                    {#each bands as option (option)}
                        <option value={option}>{t(option === "all" ? "scan_all_bands" : `band_${option}`)}</option>
                    {/each}
                </select>
            </label>
            <label>
                <span>{t("scan_sort")}</span>
                <select bind:value={sort}>
                    <option value="quality">{t("scan_quality")}</option>
                    <option value="ssid">{t("table_ssid")}</option>
                    <option value="frequency">{t("scan_frequency")}</option>
                </select>
            </label>
        </div>
    {/if}

    {#if points.length === 0}
        <p class="empty">{t("no_networks")}</p>
    {:else if visible.length === 0}
        <p class="empty">{t("scan_no_matches")}</p>
    {:else}
        <div class="table-scroll">
            <table>
                <thead>
                    <tr>
                        <th scope="col">{t("table_ssid")}</th>
                        <th scope="col" class="wide-only">{t("table_bssid")}</th>
                        <th scope="col">{t("table_band")}</th>
                        <th scope="col" class="wide-only">{t("scan_frequency")}</th>
                        <th scope="col">{t("scan_quality")}</th>
                    </tr>
                </thead>
                <tbody>
                    {#each visible as point, index (point)}
                        <tr class:active-row={point.isConnected}>
                            <th scope="row">
                                <button
                                    type="button"
                                    class="point-toggle"
                                    aria-expanded={expanded === point}
                                    aria-controls={expanded === point ? `scan-details-${index}` : undefined}
                                    onclick={() => { expanded = expanded === point ? null : point; }}
                                >
                                    <span class="chevron" aria-hidden="true">{expanded === point ? "▾" : "▸"}</span>
                                    <span class="point-name">{point.ssid || t("scan_hidden_ssid")}</span>
                                </button>
                                {#if point.isConnected}<span class="connected-label">{t("scan_connected")}</span>{/if}
                            </th>
                            <td class="wide-only identifier">{point.bssid || t("scan_metric_missing")}</td>
                            <td class="band"><span>{t(`band_${bandOf(point.frequencyMhz)}`)}</span></td>
                            <td class="wide-only frequency">{t("scan_mhz_value", [String(point.frequencyMhz)])}</td>
                            <td><SignalBadge value={point.qualityPercent} missingLabel={t("scan_metric_missing")} /></td>
                        </tr>
                        {#if expanded === point}
                            <tr id={`scan-details-${index}`} class="detail-row">
                                <td colspan="5"><PointDetails {point} /></td>
                            </tr>
                        {/if}
                    {/each}
                </tbody>
            </table>
        </div>
    {/if}
</section>

<style>
    .table-card { padding: 0; min-width: 0; overflow: hidden; }
    .section-head, .tool-row { padding: var(--space-3) var(--space-4); }
    .section-head { display: flex; justify-content: space-between; gap: var(--space-3); align-items: center; }
    h2 { margin: 0; font-size: 1.15rem; }
    p { margin: var(--space-1) 0 0; color: var(--color-text-muted); font-size: .875rem; }
    .tool-row { display: grid; grid-template-columns: minmax(0, 1fr) auto auto; gap: var(--space-2); border-top: 1px solid var(--color-border); border-bottom: 1px solid var(--color-border); }
    label { min-width: 0; display: grid; gap: var(--space-1); color: var(--color-text-muted); font-size: .75rem; }
    input, select, .secondary { min-width: 0; padding: var(--space-2); border: 1px solid var(--color-border); border-radius: var(--radius-md); color: var(--color-text); background: var(--color-bg); }
    .secondary { cursor: pointer; white-space: nowrap; }
    .table-scroll { max-block-size: min(45rem, 65dvh); overflow: auto; }
    thead th { position: sticky; top: 0; background: var(--color-surface); box-shadow: 0 1px var(--color-border); }
    th, td { padding: var(--space-2) var(--space-3); }
    th:first-child, td:first-child { padding-inline-start: var(--space-4); }
    th:last-child, td:last-child { padding-inline-end: var(--space-4); }
    th[scope="row"] { width: 33%; text-transform: none; color: var(--color-text); }
    .point-toggle { border: 0; background: transparent; cursor: pointer; color: inherit; display: inline-flex; align-items: baseline; text-align: start; gap: var(--space-1); padding: 0; max-width: 100%; font-weight: 600; }
    .point-name { overflow-wrap: anywhere; }
    .chevron, .connected-label { color: var(--color-primary); }
    .connected-label { display: block; margin-inline-start: var(--space-3); font-size: .7rem; font-weight: 500; }
    .identifier { white-space: nowrap; font: .8rem ui-monospace, monospace; color: var(--color-text-muted); }
    .frequency { white-space: nowrap; font-size: .85rem; font-variant-numeric: tabular-nums; color: var(--color-text-muted); }
    .band { white-space: nowrap; color: var(--color-text-muted); font-size: .875rem; }
    .active-row, .detail-row { background: var(--color-surface-raised); }
    .detail-row td { padding-block: var(--space-3); }
    .empty { margin: 0; padding: var(--space-4); border-top: 1px solid var(--color-border); }
    @media (max-width: 720px) {
        .wide-only { display: none; }
        th[scope="row"] { width: 48%; }
    }
    @media (max-width: 560px) {
        .tool-row { grid-template-columns: repeat(2, minmax(0, 1fr)); }
        .search { grid-column: 1 / -1; }
        .section-head, .tool-row { padding-inline: var(--space-3); }
        th, td { padding-inline: var(--space-2); }
        th:first-child, td:first-child { padding-inline-start: var(--space-3); }
        th:last-child, td:last-child { padding-inline-end: var(--space-3); }
        .table-scroll { max-block-size: none; }
    }
</style>
