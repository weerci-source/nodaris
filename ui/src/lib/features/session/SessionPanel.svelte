<script lang="ts">
    import { isTauri } from "@tauri-apps/api/core";
    import { onMount } from "svelte";
    import Alert from "../../components/ui/Alert.svelte";
    import Button from "../../components/ui/Button.svelte";
    import LoadingStatus from "../../components/ui/LoadingStatus.svelte";
    import { currentLang, t, translateError, type ApiError } from "../../i18n";
    import ConnectionResult from "../connection/ConnectionResult.svelte";
    import ProbeResult from "../probes/ProbeResult.svelte";
    import RadioResult from "../radio/RadioResult.svelte";
    import { fetchWifiInterfaces, type WifiInterface } from "../radio/radio-api";
    import { runSession, type DiagnosticSession } from "./session-api";

    const id = $props.id();
    const time = new Intl.DateTimeFormat(currentLang, { dateStyle: "short", timeStyle: "medium" });
    const number = new Intl.NumberFormat(currentLang, { maximumFractionDigits: 2 });
    const desktop = isTauri();
    const historyLimit = 10;
    let interfaces = $state.raw<WifiInterface[]>([]);
    let interfaceName = $state("");
    let target = $state("");
    let loading = $state(false);
    let listing = $state(false);
    let listError = $state<ApiError | null>(null);
    let error = $state<ApiError | null>(null);
    let targetError = $state<ApiError | null>(null);
    let history = $state.raw<DiagnosticSession[]>([]);
    let selectedId = $state("");
    let report = $derived(history.find((item) => item.id === selectedId));
    let active: AbortController | null = null;
    let listRequest: AbortController | null = null;

    async function listInterfaces() {
        if (listRequest) return;
        const request = new AbortController();
        listRequest = request;
        listError = null;
        listing = true;
        const result = await fetchWifiInterfaces();
        if (request.signal.aborted) return;
        listRequest = null;
        listing = false;
        if (!result.ok) {
            listError = result.error;
            return;
        }
        interfaces = result.value;
        if (!interfaces.some((item) => item.name === interfaceName)) {
            interfaceName = interfaces.find((item) => item.connected)?.name ?? interfaces[0]?.name ?? "";
        }
    }

    async function run(event: SubmitEvent) {
        event.preventDefault();
        if (active || !interfaceName) return;
        const request = new AbortController();
        active = request;
        error = null;
        targetError = null;
        loading = true;
        const result = await runSession(interfaceName, target.trim() || null, request.signal);
        if (request.signal.aborted) return;
        active = null;
        loading = false;
        if (result.ok) {
            history = [result.report, ...history].slice(0, historyLimit);
            selectedId = result.report.id;
        } else if (result.error.key === "diagnostics_session_invalid_target") {
            targetError = result.error;
        } else {
            error = result.error;
        }
    }

    function cancel() {
        active?.abort();
        active = null;
        loading = false;
    }

    function formatTime(value: number | null) {
        return value === null ? t("scan_metric_missing") : time.format(value);
    }

    function sectionReason(status: "unavailable" | "not_collected", reason?: string) {
        const key = `session_status_${reason ?? status}`;
        const message = t(key);
        return message === key ? t("session_status_unavailable") : message;
    }

    onMount(() => {
        void listInterfaces();
        return () => {
            active?.abort();
            listRequest?.abort();
        };
    });
</script>

<section id="diagnostic-session" class="card ui-stack ui-section" aria-labelledby={`${id}-title`}>
    <header class="ui-row ui-row-between">
        <div>
            <h2 class="ui-title" id={`${id}-title`}>{t("session_title")}</h2>
            <p class="ui-muted">{t("session_intro")}</p>
        </div>
        <Button variant="secondary" loading={listing} disabled={loading} onclick={listInterfaces}>
            {t("radio_ui_refresh_interfaces")}
        </Button>
    </header>

    {#if listError}
        <Alert severity={listError.severity}>{translateError(listError)}</Alert>
    {:else if !listing && !interfaces.length}
        <p class="ui-muted" role="status">{t("radio_ui_no_interfaces")}</p>
    {/if}

    {#if interfaces.length}
        <form class="ui-row" onsubmit={run} novalidate>
            <label class="ui-field">
                <span>{t("radio_ui_interface")}</span>
                <select class="ui-control" bind:value={interfaceName} disabled={loading || listing}>
                    {#each interfaces as item (item.name)}
                        <option value={item.name}>{item.name} — {t(item.connected ? "radio_ui_connected" : "radio_ui_disconnected")}</option>
                    {/each}
                </select>
            </label>
            <label class="ui-field">
                <span>{t("session_target")}</span>
                <input class="ui-control" type="text" inputmode="decimal" maxlength="15"
                    bind:value={target} disabled={loading} aria-invalid={Boolean(targetError)}
                    aria-describedby={targetError ? `${id}-target-error` : undefined}
                    oninput={() => { targetError = null; }} />
            </label>
            <Button type="submit" {loading} disabled={listing || !interfaceName}>{t("session_run")}</Button>
            {#if loading && !desktop}
                <Button variant="secondary" onclick={cancel}>{t("connection_cancel")}</Button>
            {/if}
            {#if targetError}
                <div class="ui-field-error" id={`${id}-target-error`}>
                    <Alert severity={targetError.severity}>
                        {translateError(targetError)}
                        {#if targetError.request_id}<p class="ui-muted">{t("session_request_id", [targetError.request_id])}</p>{/if}
                    </Alert>
                </div>
            {/if}
        </form>
    {/if}

    <LoadingStatus message={loading ? t(report ? "session_refreshing" : "session_loading") : ""} />
    {#if error}
        <Alert severity={error.severity}>
            {translateError(error)}
            {#if error.request_id}<p class="ui-muted">{t("session_request_id", [error.request_id])}</p>{/if}
        </Alert>
    {/if}

    {#if history.length}
        <label class="ui-field">
            <span>{t("session_history")}</span>
            <select class="ui-control" bind:value={selectedId}>
                {#each history as item (item.id)}
                    <option value={item.id}>{formatTime(item.started_at.unix_time_ms)} · {item.interface}</option>
                {/each}
            </select>
        </label>
    {/if}

    {#if report}
        <p class="ui-muted">{t("session_window", [
            report.interface, formatTime(report.started_at.unix_time_ms), formatTime(report.finished_at.unix_time_ms), String(report.rules_version),
        ])}</p>

        <section class="ui-stack ui-subsection" aria-labelledby={`${id}-findings`}>
            <h3 class="ui-title" id={`${id}-findings`}>{t("session_findings")}</h3>
            {#each report.hypotheses as finding (finding.key)}
                <article class="card ui-stack">
                    <h4 class="ui-title">{t(`session_finding_${finding.key}`)}</h4>
                    {#if finding.key === "beyond_gateway"}
                        <p class="ui-muted">{t("session_beyond_evidence", [
                            number.format(finding.gateway_avg_rtt_ms), number.format(finding.external_avg_rtt_ms),
                            formatTime(finding.started_at_ms), formatTime(finding.finished_at_ms),
                        ])}</p>
                        <p class="ui-muted">{t("session_beyond_limit")}</p>
                    {:else}
                        <p class="ui-muted">{t("session_sticky_evidence", [
                            finding.evidence.candidate_bssid, number.format(finding.evidence.minimum_signal_advantage_db),
                            number.format(finding.evidence.duration_ns / 1e9),
                        ])}</p>
                        <p class="ui-muted">{t("session_sticky_limit")}</p>
                    {/if}
                </article>
            {:else}
                <p class="ui-muted">{t("session_no_findings")}</p>
            {/each}
        </section>

        <section class="ui-stack ui-subsection" aria-labelledby={`${id}-radio`}>
            <h3 class="ui-title" id={`${id}-radio`}>{t("radio_ui_title")}</h3>
            {#if report.radio.status === "available"}
                <RadioResult radio={report.radio.data} />
            {:else}
                <p class="ui-muted">{sectionReason(report.radio.status, report.radio.status === "unavailable" ? report.radio.reason : undefined)}</p>
            {/if}
        </section>

        <section class="ui-stack ui-subsection" aria-labelledby={`${id}-probes`}>
            <h3 class="ui-title" id={`${id}-probes`}>{t("probe_title")}</h3>
            {#if report.probes.status === "available"}
                <ProbeResult report={report.probes.data} />
            {:else}
                <p class="ui-muted">{sectionReason(report.probes.status, report.probes.status === "unavailable" ? report.probes.reason : undefined)}</p>
            {/if}
        </section>

        <section class="ui-stack ui-subsection" aria-labelledby={`${id}-connection`}>
            <h3 class="ui-title" id={`${id}-connection`}>{t("connection_title")}</h3>
            {#if report.connection.status === "available"}
                <ConnectionResult report={report.connection.data} />
            {:else}
                <p class="ui-muted">{sectionReason(report.connection.status, report.connection.status === "unavailable" ? report.connection.reason : undefined)}</p>
            {/if}
        </section>
    {/if}
</section>
