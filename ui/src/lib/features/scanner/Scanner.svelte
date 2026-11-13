<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { translateError, t, type ApiError } from "../../i18n";
    import Alert from "../../components/ui/Alert.svelte";
    import SignalBadge from "../../components/ui/SignalBadge.svelte";

    type AccessPoint = {
        ssid: string;
        bssid: string;
        signal: number;
        frequency: number;
        noise: number | null;
        snr: number | null;
        is_active: boolean;
    };

    let aps = $state<AccessPoint[]>([]);
    let loading = $state(true);
    let error = $state<ApiError | null>(null);

    async function fetchScan() {
        loading = true;
        error = null;
        try {
            aps = await invoke<AccessPoint[]>("scan_wifi");
        } catch (e: any) {
            error =
                typeof e === "object"
                    ? e
                    : {
                          key: "scan_unknown_failure",
                          code: "Unknown",
                          severity: "error",
                          args: [],
                      };
        } finally {
            loading = false;
        }
    }

    function getBand(freq: number): string {
        if (freq >= 5900) return t("band_6g");
        if (freq >= 4900) return t("band_5g");
        if (freq >= 2400) return t("band_24g");
        return t("band_unknown");
    }

    $effect(() => {
        fetchScan();
    });
</script>

<div
    style="display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--space-4);"
>
    <button class="btn" onclick={fetchScan} disabled={loading}>
        {loading ? t("scanning_btn") : t("scan_btn")}
    </button>
</div>

{#if error}
    <Alert severity={error.severity}>{translateError(error)}</Alert>
{:else if loading}
    <div class="card">{t("scanning_air")}</div>
{:else if aps.length === 0}
    <div class="card">{t("no_networks")}</div>
{:else}
    <div class="card table-scroll" style="flex: 1; min-height: 0;">
        <table>
            <thead>
                <tr>
                    <th>{t("table_ssid")}</th>
                    <th>{t("table_bssid")}</th>
                    <th>{t("table_band")}</th>
                    <th>{t("table_signal")}</th>
                    <th>{t("table_diag")}</th>
                </tr>
            </thead>
            <tbody>
                {#each aps as ap}
                    <tr>
                        <td><strong>{ap.ssid}</strong></td>
                        <td
                            style="color: var(--color-text-muted); font-family: monospace;"
                        >
                            {ap.bssid}
                        </td>
                        <td>
                            <span
                                style="background: var(--color-bg); padding: 2px 8px; border-radius: 4px; border: 1px solid var(--color-border); font-size: 0.85em;"
                            >
                                {getBand(ap.frequency)}
                            </span>
                        </td>
                        <td><SignalBadge signal={ap.signal} /></td>
                        <td>
                            {#if ap.is_active}
                                {#if ap.snr !== null}
                                    <div
                                        style="display: flex; flex-direction: column; gap: 2px;"
                                    >
                                        <span
                                            style="color: {ap.snr > 30
                                                ? 'var(--color-success)'
                                                : ap.snr > 20
                                                  ? 'var(--color-warning)'
                                                  : 'var(--color-danger)'}; font-weight: bold;"
                                        >
                                            {t("label_snr")}
                                            {ap.snr}
                                            {t("unit_db")}
                                        </span>
                                        <span
                                            style="font-size: 0.8em; color: var(--color-text-muted);"
                                        >
                                            {t("label_noise")}
                                            {ap.noise}
                                            {t("unit_dbm")}
                                        </span>
                                    </div>
                                {:else}
                                    <span
                                        style="color: var(--color-warning); font-size: 0.85em;"
                                    >
                                        {t("label_no_noise")}
                                    </span>
                                {/if}
                            {:else}
                                <span
                                    style="color: var(--color-text-muted); font-size: 0.85em;"
                                    >—</span
                                >
                            {/if}
                        </td>
                    </tr>
                {/each}
            </tbody>
        </table>
    </div>
{/if}
