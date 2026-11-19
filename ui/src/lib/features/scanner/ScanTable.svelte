<script lang="ts">
    import { currentLang, t } from "../../i18n";
    import Button from "../../components/ui/Button.svelte";
    import Icon from "../../components/ui/Icon.svelte";
    import Skeleton from "../../components/ui/Skeleton.svelte";
    import SignalBadge from "../../components/ui/SignalBadge.svelte";
    import PointDetails from "./PointDetails.svelte";
    import { bandOf, selectPoints, type AccessPoint, type Band, type ScanSort } from "./scan-model";

    let { points, loading }: { points: AccessPoint[]; loading: boolean } = $props();
    const bands = ["all", "24g", "5g", "6g", "unknown"] as const;
    let query = $state("");
    let band = $state<Band | "all">("all");
    let sort = $state<ScanSort>("quality");
    let expanded = $state.raw<AccessPoint | null>(null);
    let searchInput: HTMLInputElement | undefined = $state();
    let visible = $derived(selectPoints(points, query, band, sort, currentLang));
    let filtered = $derived(query.trim() !== "" || band !== "all");

    function clearFilters() {
        query = "";
        band = "all";
        expanded = null;
        searchInput?.focus();
    }
</script>

<section class="card table-card" aria-labelledby="nearby-heading">
    <header class="section-head">
        <div>
            <h2 id="nearby-heading">{t("scan_nearby")}</h2>
            {#if !loading && points.length > 0}
                <p role="status">{t("scan_results_count", [String(visible.length), String(points.length)])}</p>
            {/if}
        </div>
        {#if points.length > 0 && filtered}
            <Button variant="secondary" size="sm" onclick={clearFilters}>{t("scan_clear_filters")}</Button>
        {/if}
    </header>
    {#if points.length > 0}
        <div class="tool-row">
            <label class="ui-field search">
                <span>{t("scan_search")}</span>
                <input class="ui-control" type="search" bind:this={searchInput} bind:value={query} placeholder={t("scan_search_placeholder")} />
            </label>
            <label class="ui-field">
                <span>{t("table_band")}</span>
                <select class="ui-control" bind:value={band}>
                    {#each bands as option (option)}
                        <option value={option}>{t(option === "all" ? "scan_all_bands" : `band_${option}`)}</option>
                    {/each}
                </select>
            </label>
            <label class="ui-field">
                <span>{t("scan_sort")}</span>
                <select class="ui-control" bind:value={sort}>
                    <option value="quality">{t("scan_quality")}</option>
                    <option value="ssid">{t("table_ssid")}</option>
                    <option value="frequency">{t("scan_frequency")}</option>
                </select>
            </label>
        </div>
    {/if}

    {#if !loading && points.length === 0}
        <p class="empty" role="status">{t("no_networks")}</p>
    {:else if !loading && visible.length === 0}
        <p class="empty" role="status">{t("scan_no_matches")}</p>
    {:else}
        <!-- svelte-ignore a11y_no_noninteractive_tabindex (Keyboard users need to focus the scrollable table region.) -->
        <div class="table-scroll" role="region" aria-labelledby="nearby-heading" tabindex={loading ? undefined : 0} aria-busy={loading}>
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
                {#if loading}
                    <tbody aria-hidden="true">
                        {#each [1, 2, 3] as row (row)}
                            <tr>
                                <td class="skeleton-name"><Skeleton /></td>
                                <td class="wide-only"><Skeleton /></td>
                                <td class="skeleton-band"><Skeleton /></td>
                                <td class="wide-only"><Skeleton /></td>
                                <td class="skeleton-quality"><Skeleton /></td>
                            </tr>
                        {/each}
                    </tbody>
                {:else}
                    <tbody>
                        {#each visible as point, index (point)}
                            <tr class:active-row={point.isConnected}>
                                <th scope="row">
                                    <Button
                                        variant="plain"
                                        size="sm"
                                        icon={expanded === point ? "chevron-down" : "chevron-right"}
                                        aria-expanded={expanded === point}
                                        aria-controls={expanded === point ? `scan-details-${index}` : undefined}
                                        onclick={() => { expanded = expanded === point ? null : point; }}
                                    >
                                        <span class="point-name">{point.ssid || t("scan_hidden_ssid")}</span>
                                    </Button>
                                    {#if point.isConnected}
                                        <span class="connected-label"><Icon name="check" /> {t("scan_connected")}</span>
                                    {/if}
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
                {/if}
            </table>
        </div>
    {/if}
</section>

<style>
    .table-card { padding: 0; min-width: 0; }
    .section-head, .tool-row { padding: var(--space-3) var(--space-4); }
    .section-head { display: flex; justify-content: space-between; gap: var(--space-3); align-items: center; flex-wrap: wrap; }
    h2 { margin: 0; font-size: 1.15rem; }
    p { margin: var(--space-1) 0 0; color: var(--color-text-muted); font-size: .875rem; }
    .tool-row { display: grid; grid-template-columns: minmax(0, 1fr) auto auto; gap: var(--space-2); border-top: 1px solid var(--color-border); border-bottom: 1px solid var(--color-border); }
    .table-scroll { max-block-size: min(45rem, 65dvh); scroll-padding-block-start: var(--control-height); border-end-start-radius: var(--radius-md); border-end-end-radius: var(--radius-md); }
    .table-scroll:focus-visible { outline-offset: calc(-1 * var(--space-1)); }
    .skeleton-name { --skeleton-width: 11rem; }
    .skeleton-band, .skeleton-quality { --skeleton-width: 4rem; }
    thead th { position: sticky; top: 0; background: var(--color-surface); box-shadow: 0 1px var(--color-border); }
    th, td { padding: var(--space-2) var(--space-3); }
    th:first-child, td:first-child { padding-inline-start: var(--space-4); }
    th:last-child, td:last-child { padding-inline-end: var(--space-4); }
    th[scope="row"] { width: 33%; text-transform: none; color: var(--color-text); }
    .point-name { overflow-wrap: anywhere; }
    .connected-label { display: flex; align-items: center; gap: var(--space-1); color: var(--color-success); padding-inline-start: var(--space-1); font-size: .75rem; font-weight: 500; }
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
