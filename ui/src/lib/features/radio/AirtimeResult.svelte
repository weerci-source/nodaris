<script lang="ts">
    import { Chart, LinearScale, LineController, LineElement, PointElement, Tooltip } from "chart.js";
    import { appearance } from "../../appearance.svelte";
    import { currentLang, t } from "../../i18n";
    import Button from "../../components/ui/Button.svelte";
    import type { AirtimeSeries } from "./radio-api";

    Chart.register(LinearScale, LineController, LineElement, PointElement, Tooltip);

    let { series }: { series: AirtimeSeries } = $props();
    const id = $props.id();
    let canvas = $state<HTMLCanvasElement>();
    let showSamples = $state(false);
    let format = $derived(new Intl.NumberFormat(currentLang(), { maximumFractionDigits: 2 }));
    let latest = $derived(series.samples.findLast((sample) => sample.reading.status === "available"));
    let duration = $derived((series.finished_at.monotonic_time_ns - series.started_at.monotonic_time_ns) / 1e9);
    let missing = $derived(series.samples.filter((sample) => sample.reading.status === "unavailable").length);
    let points = $derived(series.samples.map(({ elapsed_ns, reading }) => ({
        x: elapsed_ns / 1e9,
        y: reading.status === "available" ? reading.utilization_percent : null,
    })));

    function reasonText(reason: string) {
        const key = `radio_ui_survey_${reason}`;
        const message = t(key);
        return message === key ? t("radio_ui_unavailable") : message;
    }

    $effect(() => {
        if (!canvas || !latest) return;
        // Canvas rendering uses resolved CSS colors and follows changes to the system theme.
        void appearance.resolvedTheme;
        const styles = getComputedStyle(canvas);
        const color = (name: string) => styles.getPropertyValue(name).trim();
        const label = color("--color-text-muted");
        const border = color("--color-border");
        const chart = new Chart(canvas, {
            type: "line",
            data: {
                datasets: [{
                    data: points,
                    borderColor: color("--color-primary"),
                    pointBackgroundColor: color("--color-primary"),
                    pointRadius: 2,
                    borderWidth: 2,
                    clip: 4,
                    spanGaps: false,
                }],
            },
            options: {
                locale: currentLang(),
                maintainAspectRatio: false,
                animation: false,
                interaction: { intersect: false, mode: "nearest", axis: "x" },
                scales: {
                    x: {
                        type: "linear", min: 0, max: duration,
                        title: { display: true, text: t("airtime_axis_time"), color: label },
                        ticks: { color: label, maxTicksLimit: 6, includeBounds: false },
                        grid: { color: border },
                    },
                    y: {
                        min: 0, max: 100,
                        ticks: { color: label, stepSize: 25, callback: (value) => `${value}%` },
                        grid: { color: border },
                    },
                },
                plugins: {
                    tooltip: {
                        displayColors: false,
                        backgroundColor: color("--color-surface-raised"),
                        borderColor: border, borderWidth: 1,
                        titleColor: color("--color-text"), bodyColor: color("--color-text"),
                        callbacks: {
                            title: ([point]) => point?.parsed.x == null
                                ? "" : t("radio_ui_seconds_value", [format.format(point.parsed.x)]),
                            label: (point) => point.parsed.y == null ? "" : `${format.format(point.parsed.y)}%`,
                            afterLabel: (point) => {
                                const reading = series.samples[point.dataIndex].reading;
                                if (reading.status !== "available") return "";
                                return t("airtime_window_value", [format.format(reading.interval_ns / 1e9)]);
                            },
                        },
                    },
                },
            },
        });
        return () => chart.destroy();
    });
</script>

{#if series.frequency_mhz !== null}
    <dl class="ui-metrics">
        <div><dt>{t("airtime_frequency")}</dt><dd>{t("scan_mhz_value", [format.format(series.frequency_mhz)])}</dd></div>
        {#if latest?.reading.status === "available"}
            <div><dt>{t("airtime_latest")}</dt><dd>{format.format(latest.reading.utilization_percent)}%</dd></div>
            <div>
                <dt>{t("airtime_window")}</dt>
                <dd>{t("radio_ui_seconds_value", [format.format(latest.reading.interval_ns / 1e9)])}</dd>
            </div>
        {/if}
    </dl>
{/if}

{#if latest}
    <div class="airtime-chart" role="img" aria-label={t("airtime_chart_label")}>
        <canvas bind:this={canvas} aria-hidden="true"></canvas>
    </div>
    {#if missing}<p class="ui-muted">{t("airtime_missing", [format.format(missing)])}</p>{/if}
    <div>
        <Button variant="plain" size="sm" icon={showSamples ? "chevron-down" : "chevron-right"}
            aria-expanded={showSamples} aria-controls={`${id}-samples`}
            onclick={() => { showSamples = !showSamples; }}>{t("airtime_samples")}</Button>
        <div class="table-scroll" id={`${id}-samples`} hidden={!showSamples}>
            <table>
                <thead><tr><th scope="col">{t("airtime_axis_time")}</th><th scope="col">{t("radio_ui_airtime_busy_share")}</th><th scope="col">{t("airtime_window")}</th></tr></thead>
                <tbody>
                    {#each series.samples as sample (sample.elapsed_ns)}
                        <tr>
                            <td>{format.format(sample.elapsed_ns / 1e9)}</td>
                            {#if sample.reading.status === "available"}
                                <td>{format.format(sample.reading.utilization_percent)}%</td>
                                <td>{t("radio_ui_seconds_value", [format.format(sample.reading.interval_ns / 1e9)])}</td>
                            {:else}
                                <td colspan="2">{reasonText(sample.reading.reason)}</td>
                            {/if}
                        </tr>
                    {/each}
                </tbody>
            </table>
        </div>
    </div>
{:else if !series.stopped_reason}
    {@const last = series.samples.at(-1)?.reading}
    <p class="ui-muted" role="status">{last?.status === "unavailable" ? reasonText(last.reason) : t("radio_ui_unavailable")}</p>
{/if}

{#if series.stopped_reason}
    <p class="ui-muted" role="status">{reasonText(series.stopped_reason)}</p>
{/if}
