<script lang="ts">
    import { onMount, tick } from "svelte";
    import Alert from "../../components/ui/Alert.svelte";
    import Button from "../../components/ui/Button.svelte";
    import LoadingStatus from "../../components/ui/LoadingStatus.svelte";
    import Skeleton from "../../components/ui/Skeleton.svelte";
    import { t, translateError, type ApiError } from "../../i18n";
    import ProbeResult from "../probes/ProbeResult.svelte";
    import { listEthernetLinks, runEthernet, type EthernetLink, type EthernetReport } from "./ethernet-api";

    import EthernetLinkResult from "./EthernetLinkResult.svelte";

    let { onSaved }: { onSaved: () => void } = $props();
    const id = $props.id();
    let links = $state.raw<EthernetLink[] | null>(null);
    let interfaceName = $state("");
    let link = $derived(links?.find((item) => item.interface === interfaceName));
    let target = $state("");
    let targetInput = $state<HTMLInputElement>();
    let saveError = $state<ApiError | null>(null);
    let listing = $state(true);
    let running = $state(false);
    let listError = $state<ApiError | null>(null);
    let runError = $state<ApiError | null>(null);
    let targetError = $state<ApiError | null>(null);
    let lastReport = $state.raw<EthernetReport | null>(null);
    let report = $derived(lastReport?.link.interface === interfaceName ? lastReport : null);
    let request: AbortController | null = null;

    async function refresh() {
        if (request) return;
        const controller = new AbortController();
        request = controller;
        listing = true;
        listError = null;
        const result = await listEthernetLinks(controller.signal);
        if (controller.signal.aborted) return;
        request = null;
        listing = false;
        if (!result.ok) {
            listError = result.error;
            return;
        }
        links = result.value;
        if (!links.some((item) => item.interface === interfaceName)) {
            interfaceName = links.find((item) => item.state.state.status === "available" &&
                item.state.state.value === "up")?.interface ?? links[0]?.interface ?? "";
            lastReport = null;
            saveError = null;
            runError = null;
        }
    }

    async function run(event: SubmitEvent) {
        event.preventDefault();
        if (request || !link) return;
        const controller = new AbortController();
        request = controller;
        running = true;
        runError = null;
        targetError = null;
        const result = await runEthernet(interfaceName, target.trim(), controller.signal);
        if (controller.signal.aborted) return;
        request = null;
        running = false;
        if (result.ok) {
            lastReport = result.value;
            saveError = result.historyError;
            links = links?.map((item) => item.interface === result.value.link.interface
                ? result.value.link : item) ?? null;
            if (!saveError) onSaved();
        } else if (result.error.key === "diagnostics_ethernet_invalid_target" ||
            result.error.key === "diagnostics_ethernet_target_unresolved") {
            targetError = result.error;
        } else {
            runError = result.error;
        }
    }

    export async function prepareRepeat(name: string, address: string, reveal: () => void): Promise<boolean> {
        if (request || !links?.some((item) => item.interface === name)) return false;
        interfaceName = name;
        target = address;
        targetError = null;
        runError = null;
        reveal();
        await tick();
        targetInput?.focus();
        return true;
    }

    onMount(() => {
        void refresh();
        return () => request?.abort();
    });
</script>

{#snippet showError(error: ApiError)}
    <Alert severity={error.severity}>
        {translateError(error)}
        {#if error.request_id}
            <p class="ui-muted">{t("session_request_id", [error.request_id])}</p>
        {/if}
    </Alert>
{/snippet}

<section id="ethernet" class="card ui-stack ui-section" aria-labelledby={`${id}-title`}>
    <header class="ui-row ui-row-between">
        <h2 class="ui-title" id={`${id}-title`}>{t("ethernet_title")}</h2>
        <Button variant="secondary" loading={listing} disabled={running} onclick={refresh}>
            {t("radio_ui_refresh_interfaces")}
        </Button>
    </header>

    {#if listing && links === null}<Skeleton />{/if}
    <LoadingStatus message={listing
        ? t(links === null ? "ethernet_loading" : "ethernet_refreshing")
        : running ? t(report ? "probe_refreshing" : "probe_collecting") : ""} />
    {#if listError}
        {@render showError(listError)}
    {:else if links?.length === 0}
        <p class="ui-muted" role="status">{t("ethernet_empty")}</p>
    {/if}

    {#if link && links}
        <form class="ui-stack" onsubmit={run} novalidate>
            <div class="ui-row">
                <label class="ui-field">
                    <span>{t("ethernet_interface")}</span>
                    <select class="ui-control" bind:value={interfaceName} disabled={listing || running}
                        onchange={() => { runError = null; }}>
                        {#each links as item (item.interface)}
                            <option value={item.interface}>{item.interface}</option>
                        {/each}
                    </select>
                </label>
                <div class="ui-stack">
                    <label class="ui-field">
                        <span>{t("ethernet_target")}</span>
                        <input class="ui-control" type="text" maxlength="254" autocapitalize="none" spellcheck={false} required
                            bind:this={targetInput} bind:value={target} disabled={running} aria-invalid={Boolean(targetError)}
                            aria-describedby={targetError ? `${id}-target-error` : undefined}
                            oninput={() => { targetError = null; }} />
                    </label>
                    {#if targetError}
                        <div id={`${id}-target-error`}>{@render showError(targetError)}</div>
                    {/if}
                </div>
                <Button type="submit" loading={running} disabled={listing}>{t("probe_start")}</Button>
            </div>
            <EthernetLinkResult {link} />
        </form>
    {/if}

    {#if runError}{@render showError(runError)}{/if}
    {#if report}
        {#if saveError}{@render showError(saveError)}{/if}
        <section class="ui-stack ui-subsection" aria-labelledby={`${id}-probes`}>
            <h3 class="ui-title" id={`${id}-probes`}>{t("probe_title")}</h3>
            <ProbeResult report={report.probes} hostname={report.hostname} />
        </section>
    {/if}
</section>
