<script lang="ts">
    import { isTauri } from "@tauri-apps/api/core";
    import { onMount, tick } from "svelte";
    import Alert from "../../components/ui/Alert.svelte";
    import Button from "../../components/ui/Button.svelte";
    import LoadingStatus from "../../components/ui/LoadingStatus.svelte";
    import { t, translateError, type ApiError } from "../../i18n";
    import { fetchWifiInterfaces, type WifiInterface } from "../radio/radio-api";
    import { runSession, type DiagnosticSession } from "./session-api";

    import SessionResult from "./SessionResult.svelte";

    let { onSaved }: { onSaved: () => void } = $props();
    const id = $props.id();
    const desktop = isTauri();
    let interfaces = $state.raw<WifiInterface[]>([]);
    let interfaceName = $state("");
    let target = $state("");
    let loading = $state(false);
    let listing = $state(false);
    let listError = $state<ApiError | null>(null);
    let error = $state<ApiError | null>(null);
    let targetError = $state<ApiError | null>(null);
    let report = $state.raw<DiagnosticSession | null>(null);
    let saveError = $state<ApiError | null>(null);
    let targetInput = $state<HTMLInputElement>();
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
            report = result.report;
            saveError = result.historyError;
            if (!saveError) onSaved();
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

    export async function prepareRepeat(name: string, address: string | null): Promise<boolean> {
        if (loading || listing || !interfaces.some((item) => item.name === name)) return false;
        interfaceName = name;
        target = address ?? "";
        targetError = null;
        error = null;
        await tick();
        targetInput?.focus();
        return true;
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
                    bind:this={targetInput} bind:value={target} disabled={loading} aria-invalid={Boolean(targetError)}
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

    {#if saveError}
        <Alert severity={saveError.severity}>
            {translateError(saveError)}
            {#if saveError.request_id}<p class="ui-muted">{t("session_request_id", [saveError.request_id])}</p>{/if}
        </Alert>
    {/if}
    {#if report}<SessionResult {report} />{/if}
</section>
