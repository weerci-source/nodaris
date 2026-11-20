import { invoke, isTauri } from "@tauri-apps/api/core";
import { toApiError, type ApiError } from "../../i18n";

export type WifiInterface = { name: string; connected: boolean };
type Unavailable = { status: "unavailable"; reason: string };
type Snr = Unavailable | { status: "available"; db: number };
type Noise = Unavailable | { status: "available"; dbm: number; source: string };
type Airtime = Unavailable | { status: "not_collected" } | {
    status: "available";
    utilization_percent: number;
    busy_delta_us: number;
    total_delta_us: number;
    interval_ns: number;
};
export type AirtimeSample = { elapsed_ns: number; reading: Exclude<Airtime, { status: "not_collected" }> };
export type AirtimeSeries = {
    interface: string;
    frequency_mhz: number | null;
    source: string;
    started_at: { monotonic_time_ns: number };
    finished_at: { monotonic_time_ns: number };
    samples: AirtimeSample[];
    stopped_reason: string | null;
};
type Retries = Unavailable | { status: "not_collected" } | {
    status: "available";
    retries_delta: number | null;
    failed_delta: number | null;
    attempts_delta: number | null;
    retry_rate_percent: number | null;
    interval_ns: number;
};
type RadioReading = Unavailable | {
    status: "available";
    frequency_mhz: number | null;
    signal_quality_percent: number | null;
    rssi_dbm?: number | null;
    rssi_source?: string | null;
    noise?: Noise;
    snr: Snr;
};
export type Radio = RadioReading & { retries?: Retries; airtime?: Airtime };

type Result<T> = { ok: true; value: T } | { ok: false; error: ApiError };

function isRecord(value: unknown): value is Record<string, unknown> {
    return value !== null && typeof value === "object" && !Array.isArray(value);
}

function isInteger(value: unknown): value is number {
    return typeof value === "number" && Number.isSafeInteger(value);
}

function isUnavailable(value: unknown): value is Unavailable {
    return isRecord(value) && value.status === "unavailable" &&
        typeof value.reason === "string" && value.reason.length > 0;
}

function isSnr(value: unknown): value is Snr {
    return isUnavailable(value) ||
        (isRecord(value) && value.status === "available" && isInteger(value.db));
}

function isNoise(value: unknown): value is Noise {
    return isUnavailable(value) ||
        (isRecord(value) && value.status === "available" && isInteger(value.dbm) &&
            value.dbm < 0 && typeof value.source === "string" && value.source.length > 0);
}

function isAirtime(value: unknown): value is Airtime {
    if (isUnavailable(value)) return true;
    if (!isRecord(value)) return false;
    if (value.status === "not_collected") return true;
    const { utilization_percent: percent, busy_delta_us: busy, total_delta_us: total, interval_ns: interval } = value;
    return value.status === "available" && typeof percent === "number" && percent >= 0 && percent <= 100 &&
        isInteger(total) && total > 0 && isInteger(busy) && busy >= 0 && busy <= total &&
        isInteger(interval) && interval > 0;
}

function isRadio(value: unknown): value is RadioReading {
    if (isUnavailable(value)) return true;
    if (!isRecord(value) || value.status !== "available") return false;
    const { frequency_mhz: frequency, signal_quality_percent: quality, rssi_dbm: rssi } = value;
    return (frequency === null || (isInteger(frequency) && frequency > 0)) &&
        (quality === null || (isInteger(quality) && quality >= 0 && quality <= 100)) &&
        (rssi === undefined || rssi === null || (isInteger(rssi) && rssi < 0)) &&
        (value.rssi_source == null || typeof value.rssi_source === "string") &&
        (value.noise === undefined || isNoise(value.noise)) &&
        isSnr(value.snr);
}

function isRetries(value: unknown): value is Retries {
    if (isUnavailable(value)) return true;
    if (!isRecord(value)) return false;
    if (value.status === "not_collected") return true;
    const { interval_ns: interval, retry_rate_percent: rate } = value;
    return value.status === "available" && isInteger(interval) && interval > 0 &&
        [value.retries_delta, value.failed_delta, value.attempts_delta]
            .every((delta) => delta === null || (isInteger(delta) && delta >= 0)) &&
        (rate === null || (isInteger(rate) && rate >= 0 && rate <= 100));
}

function isWifiInterface(value: unknown): value is WifiInterface {
    return isRecord(value) && typeof value.name === "string" &&
        /^[a-zA-Z0-9_.-]{1,15}$/.test(value.name) &&
        !value.name.startsWith("-") && value.name !== "." && value.name !== ".." &&
        typeof value.connected === "boolean";
}

function failure(key: string, code: string): Result<never> {
    return { ok: false, error: { key, code, severity: "error", args: [] } };
}

async function get<T>(path: string, signal: AbortSignal | undefined, decode: (body: unknown) => T | null): Promise<Result<T>> {
    let response: Response;
    try {
        response = await fetch(path, { signal, headers: { Accept: "application/json" }, cache: "no-store" });
    } catch {
        return failure("radio_ui_transport_failure", "NetworkError");
    }

    let body: unknown;
    try {
        body = await response.json();
    } catch {
        return failure("radio_ui_invalid_response", "ParseError");
    }

    if (!response.ok || (isRecord(body) && body.ok === false)) {
        if (isRecord(body) && body.ok === false && isRecord(body.error)) {
            const error = toApiError(body);
            if (error.key === body.error.key) return { ok: false, error };
        }
        return failure("radio_ui_invalid_response", "ParseError");
    }

    const value = decode(body);
    return value === null
        ? failure("radio_ui_invalid_response", "ParseError")
        : { ok: true, value };
}

function decodeInterfaces(items: unknown): WifiInterface[] | null {
    if (!Array.isArray(items) || !items.every(isWifiInterface)) return null;
    return new Set(items.map((item) => item.name)).size === items.length ? items : null;
}

async function readWifiInterfaces(): Promise<Result<WifiInterface[]>> {
    if (isTauri()) {
        try {
            const value = decodeInterfaces(await invoke<unknown>("wifi_interfaces"));
            return value === null ? failure("radio_ui_invalid_response", "ParseError") : { ok: true, value };
        } catch (error) {
            return { ok: false, error: toApiError(error, {key: "session_ipc_failure", code: "IOError", severity: "error", args: []}) };
        }
    }
    return get("/api/diagnostics/interfaces", undefined, (body) =>
        isRecord(body) ? decodeInterfaces(body.interfaces) : null);
}

let interfaceRequest: Promise<Result<WifiInterface[]>> | null = null;

export function fetchWifiInterfaces(): Promise<Result<WifiInterface[]>> {
    interfaceRequest ??= readWifiInterfaces().finally(() => { interfaceRequest = null; });
    return interfaceRequest;
}

export function decodeRadio(body: unknown, name: string): Radio | null {
    if (!isRecord(body) || body.interface !== name || !isRadio(body.radio)) return null;
    if (body.retries !== undefined && !isRetries(body.retries)) return null;
    if (body.airtime !== undefined && !isAirtime(body.airtime)) return null;
    return { ...body.radio, retries: body.retries, airtime: body.airtime };
}

export function fetchRadioDiagnostic(name: string, signal: AbortSignal): Promise<Result<Radio>> {
    const query = new URLSearchParams({ interface: name });
    return get(`/api/diagnostics/radio?${query}`, signal, (body) => decodeRadio(body, name));
}

function isObservation(value: unknown): value is AirtimeSeries["started_at"] {
    return isRecord(value) && isInteger(value.monotonic_time_ns) && value.monotonic_time_ns >= 0;
}

function isAirtimeSample(value: unknown): value is AirtimeSample {
    return isRecord(value) && isInteger(value.elapsed_ns) && value.elapsed_ns >= 0 &&
        isAirtime(value.reading) && value.reading.status !== "not_collected";
}

function isAirtimeSeries(value: unknown, name: string): value is AirtimeSeries {
    if (!isRecord(value) || value.interface !== name || typeof value.source !== "string" ||
        (value.frequency_mhz !== null && (!isInteger(value.frequency_mhz) || value.frequency_mhz <= 0)) ||
        (value.stopped_reason !== null && typeof value.stopped_reason !== "string") ||
        !isObservation(value.started_at) || !isObservation(value.finished_at) || !Array.isArray(value.samples) ||
        !value.samples.every(isAirtimeSample)) return false;
    const duration = value.finished_at.monotonic_time_ns - value.started_at.monotonic_time_ns;
    return duration >= 0 && value.samples.every((sample, index, samples) =>
        sample.elapsed_ns <= duration && (index === 0 || sample.elapsed_ns > samples[index - 1].elapsed_ns));
}

export async function observeAirtime(name: string, signal: AbortSignal): Promise<Result<AirtimeSeries>> {
    const invalid: ApiError = { key: "airtime_invalid_response", code: "ParseError", severity: "error", args: [] };
    let body: unknown;
    if (isTauri()) {
        try {
            body = await invoke("observe_airtime", { interface: name });
        } catch (cause) {
            return { ok: false, error: toApiError(cause, { key: "airtime_ipc_failure", code: "IOError", severity: "error", args: [] }) };
        }
    } else {
        let response: Response;
        try {
            response = await fetch("/api/diagnostics/airtime", {
                method: "POST",
                headers: { "Content-Type": "application/json", Accept: "application/json" },
                body: JSON.stringify({ interface: name }),
                signal,
            });
        } catch {
            return failure("airtime_http_failure", "NetworkError");
        }
        try {
            body = await response.json();
        } catch {
            return { ok: false, error: invalid };
        }
        if (!response.ok) return { ok: false, error: toApiError(body, invalid) };
    }
    return isAirtimeSeries(body, name) ? { ok: true, value: body } : { ok: false, error: invalid };
}
