<script lang="ts">
    import NetworkProbes from "../probes/NetworkProbes.svelte";
    import { onMount } from "svelte";
    import Button from "../../components/ui/Button.svelte";
    import Icon from "../../components/ui/Icon.svelte";
    import LoadingStatus from "../../components/ui/LoadingStatus.svelte";
    import SignalBadge from "../../components/ui/SignalBadge.svelte";
    import Skeleton from "../../components/ui/Skeleton.svelte";
    import Alert from "../../components/ui/Alert.svelte";
    import { t, translateError, type ApiError } from "../../i18n";
    import { fetchRadioDiagnostic, fetchWifiInterfaces, type Radio, type WifiInterface } from "./radio-api";

    let interfaces = $state<WifiInterface[]>([]);
    let selected = $state("");
    let radio = $state<Radio | null>(null);
    let listing = $state(true);
    let reading = $state(false);
    let loadingMessage = $derived(listing
        ? t("radio_ui_loading_interfaces")
        : reading ? t("radio_ui_loading_reading") : "");
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
        if (name !== selected) resetRadio();
        else radioRequest?.abort();
        radioError = null;
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
        const previous = selected;
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
            return;
        }

        interfaces = result.value;
        const connected = interfaces.filter((item) => item.connected);
        const next = interfaces.some((item) => item.name === previous)
            ? previous
            : connected.length === 1 ? connected[0].name : "";
        void readRadio(next);
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

<section class="card ui-stack ui-section" aria-labelledby="radio-heading">
    <header class="ui-row ui-row-between">
        <div>
            <h2 id="radio-heading" class="ui-title">{t("radio_ui_title")}</h2>
            <p class="ui-muted">{t("radio_ui_intro")}</p>
        </div>
        <Button variant="secondary" icon="refresh" loading={listing} disabled={reading} onclick={refreshInterfaces}>
            {t("radio_ui_refresh_interfaces")}
        </Button>
    </header>

    <LoadingStatus message={loadingMessage} />

    {#if listError}
        <Alert severity={listError.severity}>{translateError(listError)}</Alert>
    {/if}

    {#if interfaces.length}
        <div class="ui-row">
            <label class="ui-field">
                <span>{t("radio_ui_interface")}</span>
                <select class="ui-control" value={selected} disabled={listing}
                    onchange={(event) => void readRadio(event.currentTarget.value)}>
                    <option value="">{t("radio_ui_choose")}</option>
                    {#each interfaces as item (item.name)}
                        <option value={item.name}>{item.name} — {t(item.connected ? "radio_ui_connected" : "radio_ui_disconnected")}</option>
                    {/each}
                </select>
            </label>
            <Button variant="secondary" icon="refresh" loading={reading} disabled={!selected || listing} onclick={() => void readRadio(selected)}>
                {t("radio_ui_refresh_reading")}
            </Button>
        </div>

        {#if radioError}
            <Alert severity={radioError.severity}>{translateError(radioError)}</Alert>
        {/if}
        {#if (radioError || listError) && radio?.status === "available"}
            <p class="ui-muted">{t("radio_ui_previous_results")}</p>
        {/if}
        {#if reading && !radio}
            <div class="ui-metrics" aria-hidden="true">
                {#each [1, 2, 3, 4] as metric (metric)}<Skeleton />{/each}
            </div>
        {:else if radio?.status === "unavailable"}
            <p class="ui-muted" role="status"><Icon name="warning" /> {reasonText(radio.reason)}</p>
        {:else if radio?.status === "available"}
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
                </div>
                <div>
                    <dt>SNR</dt>
                    <dd>{radio.snr.status === "available" ? `${radio.snr.db} ${t("unit_db")}` : reasonText(radio.snr.reason)}</dd>
                </div>
            </dl>
        {/if}
    {:else if !listing && !listError}
        <p class="ui-muted" role="status">{t("radio_ui_no_interfaces")}</p>
    {/if}
    {#if selected}
        {#key selected}
            <NetworkProbes wifiInterface={selected} />
        {/key}
    {/if}
</section>
