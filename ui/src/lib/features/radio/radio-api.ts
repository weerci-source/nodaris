import { toApiError, type ApiError } from "../../i18n";

export type WifiInterface = { name: string; connected: boolean };
type Unavailable = { status: "unavailable"; reason: string };
type Snr = Unavailable | { status: "available"; db: number };
export type Radio = Unavailable | {
    status: "available";
    frequency_mhz: number | null;
    signal_quality_percent: number | null;
    rssi_dbm?: number | null;
    snr: Snr;
};

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

function isRadio(value: unknown): value is Radio {
    if (isUnavailable(value)) return true;
    if (!isRecord(value) || value.status !== "available") return false;
    const { frequency_mhz: frequency, signal_quality_percent: quality, rssi_dbm: rssi } = value;
    return (frequency === null || (isInteger(frequency) && frequency > 0)) &&
        (quality === null || (isInteger(quality) && quality >= 0 && quality <= 100)) &&
        (rssi === undefined || rssi === null || (isInteger(rssi) && rssi < 0)) &&
        isSnr(value.snr);
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

async function get<T>(path: string, signal: AbortSignal, decode: (body: unknown) => T | null): Promise<Result<T>> {
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

export function fetchWifiInterfaces(signal: AbortSignal): Promise<Result<WifiInterface[]>> {
    return get("/api/diagnostics/interfaces", signal, (body) => {
        if (!isRecord(body) || !Array.isArray(body.interfaces)) return null;
        const items: unknown[] = body.interfaces;
        if (!items.every(isWifiInterface)) return null;
        return new Set(items.map((item) => item.name)).size === items.length ? items : null;
    });
}

export function fetchRadioDiagnostic(name: string, signal: AbortSignal): Promise<Result<Radio>> {
    const query = new URLSearchParams({ interface: name });
    return get(`/api/diagnostics/radio?${query}`, signal, (body) =>
        isRecord(body) && body.interface === name && isRadio(body.radio) ? body.radio : null);
}
