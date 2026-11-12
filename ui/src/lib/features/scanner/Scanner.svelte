<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { translateError, type ApiError } from "../../i18n";
    import Alert from "../../components/ui/Alert.svelte";
    import SignalBadge from "../../components/ui/SignalBadge.svelte";

    type AccessPoint = {
        ssid: string;
        bssid: string;
        signal: number;
        frequency: number;
    };

    let aps = $state<AccessPoint[]>([]);
    let loading = $state(true);
    let error = $state<ApiError | null>(null);

    async function fetchScan() {
        loading = true;
        error = null;
        try {
            // Вызов Rust через Tauri IPC
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
        if (freq >= 5900) return "6 GHz";
        if (freq >= 4900) return "5 GHz";
        if (freq >= 2400) return "2.4 GHz";
        return "Unknown";
    }

    $effect(() => {
        fetchScan();
    });
</script>

<div
    style="display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--space-4);"
>
    <button class="btn" onclick={fetchScan} disabled={loading}>
        {loading ? "Scanning..." : "Scan Again"}
    </button>
</div>

{#if error}
    <Alert severity={error.severity}>{translateError(error)}</Alert>
{:else if loading}
    <div class="card">Сканирование эфира...</div>
{:else if aps.length === 0}
    <div class="card">Сети не найдены.</div>
{:else}
    <div class="card table-scroll" style="flex: 1; min-height: 0;">
        <table>
            <thead>
                <tr>
                    <th>SSID</th>
                    <th>BSSID (MAC)</th>
                    <th>Диапазон</th>
                    <th>Сигнал</th>
                </tr>
            </thead>
            <tbody>
                {#each aps as ap}
                    <tr>
                        <td><strong>{ap.ssid}</strong></td>
                        <td
                            style="color: var(--color-text-muted); font-family: monospace;"
                            >{ap.bssid}</td
                        >
                        <td>
                            <span
                                style="background: var(--color-bg); padding: 2px 8px; border-radius: 4px; border: 1px solid var(--color-border); font-size: 0.85em;"
                            >
                                {getBand(ap.frequency)}
                            </span>
                        </td>
                        <td><SignalBadge signal={ap.signal} /></td>
                    </tr>
                {/each}
            </tbody>
        </table>
    </div>
{/if}
