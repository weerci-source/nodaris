<script lang="ts">
    import { onMount } from "svelte";
    import Alert from "../../components/ui/Alert.svelte";
    import { t, translateError, type ApiError } from "../../i18n";
    import { fetchRadioDiagnostic, fetchWifiInterfaces, type Radio, type WifiInterface } from "./radio-api";

    let interfaces = $state<WifiInterface[]>([]);
    let selected = $state("");
    let radio = $state<Radio | null>(null);
    let listing = $state(true);
    let reading = $state(false);
    let listError = $state<ApiError | null>(null);
    let radioError = $state<ApiError | null>(null);
    let listRequest: AbortController | null = null;
    let radioRequest: AbortController | null = null;

    function resetRadio() {
        radioRequest?.abort();
        radioRequest = null;
        radio = null;
        radioError = null;
        reading = false;
    }

    async function readRadio(name: string) {
        resetRadio();
        selected = name;
        if (!name) return;

        const request = new AbortController();
        radioRequest = request;
        reading = true;
        const result = await fetchRadioDiagnostic(name, request.signal);
        if (radioRequest !== request || request.signal.aborted) return;
        radioRequest = null;
        reading = false;
        if (result.ok) radio = result.value;
        else radioError = result.error;
    }

    async function refreshInterfaces() {
        listRequest?.abort();
        resetRadio();
        const previous = selected;
        selected = "";
        listError = null;
        listing = true;
        const request = new AbortController();
        listRequest = request;
        const result = await fetchWifiInterfaces(request.signal);
        if (listRequest !== request || request.signal.aborted) return;
        listRequest = null;
        listing = false;

        if (!result.ok) {
            listError = result.error;
            interfaces = [];
            return;
        }

        interfaces = result.value;
        const connected = interfaces.filter((item) => item.connected);
        const next = interfaces.some((item) => item.name === previous)
            ? previous
            : connected.length === 1 ? connected[0].name : "";
        if (next) void readRadio(next);
    }

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

    onMount(() => {
        void refreshInterfaces();
        return () => {
            listRequest?.abort();
            radioRequest?.abort();
        };
    });
</script>

<section class="card diagnostics" aria-labelledby="radio-heading">
    <div class="heading">
        <div>
            <h2 id="radio-heading">{t("radio_ui_title")}</h2>
            <p>{t("radio_ui_intro")}</p>
        </div>
        <button type="button" class="btn secondary" disabled={listing || reading} onclick={refreshInterfaces}>
            {t("radio_ui_refresh_interfaces")}
        </button>
    </div>

    {#if listError}
        <Alert severity={listError.severity}>{translateError(listError)}</Alert>
    {:else if listing}
        <p role="status">{t("radio_ui_loading_interfaces")}</p>
    {:else if interfaces.length === 0}
        <p role="status">{t("radio_ui_no_interfaces")}</p>
    {:else}
        <div class="toolbar">
            <label>
                <span>{t("radio_ui_interface")}</span>
                <select value={selected} onchange={(event) => void readRadio(event.currentTarget.value)}>
                    <option value="">{t("radio_ui_choose")}</option>
                    {#each interfaces as item (item.name)}
                        <option value={item.name}>{item.name} — {t(item.connected ? "radio_ui_connected" : "radio_ui_disconnected")}</option>
                    {/each}
                </select>
            </label>
            <button type="button" class="btn secondary" disabled={!selected || reading} onclick={() => void readRadio(selected)}>
                {t("radio_ui_refresh_reading")}
            </button>
        </div>

        {#if radioError}
            <Alert severity={radioError.severity}>{translateError(radioError)}</Alert>
        {:else if reading}
            <p role="status">{t("radio_ui_loading_reading")}</p>
        {:else if radio?.status === "unavailable"}
            <p role="status">{reasonText(radio.reason)}</p>
        {:else if radio?.status === "available"}
            <dl aria-live="polite">
                <div>
                    <dt>{t("radio_ui_frequency")}</dt>
                    <dd>{radio.frequency_mhz === null ? t("radio_ui_unavailable") : t("radio_ui_mhz", [String(radio.frequency_mhz)])}</dd>
                </div>
                <div>
                    <dt>{t("radio_ui_quality")}</dt>
                    <dd>{radio.signal_quality_percent === null ? t("radio_ui_unavailable") : `${radio.signal_quality_percent}%`}</dd>
                </div>
                <div>
                    <dt>SNR</dt>
                    <dd>{radio.snr.status === "available" ? `${radio.snr.db} ${t("unit_db")}` : reasonText(radio.snr.reason)}</dd>
                </div>
            </dl>
        {/if}
    {/if}
</section>

<style>
    .diagnostics { margin-block-start: var(--space-4); }
    .heading, .toolbar { display: flex; align-items: end; flex-wrap: wrap; column-gap: var(--space-3); row-gap: var(--space-2); }
    .heading { align-items: start; justify-content: space-between; }
    .toolbar { margin-block-start: var(--space-3); }
    h2 { margin: 0; font-size: 1.2rem; }
    p { margin-block: var(--space-2) 0; color: var(--color-text-muted); }
    p[role="status"], .heading:has(+ :global(.alert)), .toolbar:has(+ :global(.alert)) { margin-block-end: var(--space-2); }
    label { display: grid; gap: var(--space-1); color: var(--color-text-muted); }
    select, .secondary { min-height: 2.5rem; border: 1px solid var(--color-border); border-radius: var(--radius-md); }
    select { color: var(--color-text); background: var(--color-bg); padding-inline: var(--space-2); max-width: 100%; }
    .secondary { color: var(--color-text); background: var(--color-bg); padding-inline: var(--space-3); }
    .secondary:hover:not(:disabled) { color: var(--color-text); background: var(--color-surface-raised); }
    .secondary:disabled { cursor: default; }
    dl { display: grid; grid-template-columns: repeat(auto-fit, minmax(min(100%, 11rem), 1fr)); gap: var(--space-3); margin-block: var(--space-3) 0; }
    dt { color: var(--color-text-muted); }
    dd { margin: var(--space-1) 0 0; }
</style>
