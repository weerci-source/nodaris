<script lang="ts">
    import ProbeResult from "./ProbeResult.svelte";
    import { onDestroy } from "svelte";
    import Alert from "../../components/ui/Alert.svelte";
    import Button from "../../components/ui/Button.svelte";
    import LoadingStatus from "../../components/ui/LoadingStatus.svelte";
    import { t, translateError, type ApiError } from "../../i18n";
    import { collectProbes, probeError, type ProbeReport } from "./probe-api";

    let { wifiInterface }: { wifiInterface: string } = $props();

    const id = $props.id();
    let target = $state("");
    let report = $state.raw<ProbeReport | null>(null);
    let error = $state<ApiError | null>(null);
    let targetError = $state<ApiError | null>(null);
    let loading = $state(false);
    let active: AbortController | null = null;

    async function run(event: SubmitEvent) {
        event.preventDefault();
        if (active) return;
        error = null;
        targetError = null;
        const address = target.trim();
        if (!/^(?:0|[1-9]\d{0,2})(?:\.(?:0|[1-9]\d{0,2})){3}$/.test(address) ||
            address.split(".").some((octet) => Number(octet) > 255)) {
            targetError = probeError("diagnostics_probe_invalid_request", "ValidationError");
            return;
        }

        const controller = new AbortController();
        active = controller;
        loading = true;
        const result = await collectProbes(wifiInterface, address, controller.signal);
        if (controller.signal.aborted) return;
        active = null;
        loading = false;
        if (result.ok) report = result.report;
        else if (result.error.key === "diagnostics_probe_invalid_request") targetError = result.error;
        else error = result.error;
    }

    onDestroy(() => active?.abort());
</script>

<section class="ui-stack ui-subsection" aria-labelledby={`${id}-heading`}>
    <header>
        <h3 class="ui-title" id={`${id}-heading`}>{t("probe_title")}</h3>
        <p class="ui-muted">{t("probe_intro")}</p>
    </header>

    <form class="ui-row" onsubmit={run} novalidate>
        <label class="ui-field" for={`${id}-target`}>
            <span>{t("probe_target")}</span>
            <input
                id={`${id}-target`}
                class="ui-control"
                type="text"
                inputmode="decimal"
                maxlength="15"
                required
                disabled={loading}
                bind:value={target}
                aria-invalid={Boolean(targetError)}
                aria-describedby={targetError ? `${id}-error` : undefined}
                oninput={() => { targetError = null; }}
            />
        </label>
        <Button type="submit" {loading}>{t("probe_start")}</Button>
        {#if targetError}
            <div class="ui-field-error" id={`${id}-error`}>
                <Alert severity={targetError.severity}>{translateError(targetError)}</Alert>
            </div>
        {/if}
    </form>

    <LoadingStatus message={loading ? t(report ? "probe_refreshing" : "probe_collecting") : ""} />

    {#if error}
        <Alert severity={error.severity}>{translateError(error)}</Alert>
    {/if}

    {#if report}<ProbeResult {report} />{/if}
</section>
