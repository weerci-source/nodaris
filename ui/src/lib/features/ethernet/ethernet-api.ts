import { invoke, isTauri } from "@tauri-apps/api/core";
import { toApiError, type ApiError } from "../../i18n";
import { historySaveError } from "../history/history-save";
import { isProbeReport, type ProbeReport } from "../probes/probe-api";

type LinkState = "up" | "down" | "disabled";
type Measurement<T> = {
    state: { status: "available"; value: T } | { status: "unavailable"; value: string };
    observed_at: { unix_time_ms: number | null };
    source: string;
};
export type EthernetLink = {
    interface: string;
    state: Measurement<LinkState>;
    speed_mbps: Measurement<number>;
};
export type EthernetReport = { link: EthernetLink; probes: ProbeReport; hostname?: string };
type Result<T> = { ok: true; value: T; historyError: ApiError | null } | { ok: false; error: ApiError };
type Request = { interface: string; target: string };

function record(value: unknown): value is Record<string, unknown> {
    return value !== null && typeof value === "object" && !Array.isArray(value);
}

function isMeasurement<T>(
    value: unknown,
    isValue: (sample: unknown) => sample is T,
): value is Measurement<T> {
    if (!record(value) || typeof value.source !== "string" || !record(value.observed_at) ||
        !record(value.state)) return false;
    const time = value.observed_at.unix_time_ms;
    if (time !== null && (typeof time !== "number" || !Number.isFinite(time) || Math.abs(time) > 8.64e15)) return false;
    return value.state.status === "available" ? isValue(value.state.value) :
        value.state.status === "unavailable" && typeof value.state.value === "string";
}

function isLink(value: unknown): value is EthernetLink {
    return record(value) && typeof value.interface === "string" &&
        isMeasurement(value.state, (sample): sample is LinkState =>
            sample === "up" || sample === "down" || sample === "disabled") &&
        isMeasurement(value.speed_mbps, (sample): sample is number =>
            typeof sample === "number" && Number.isSafeInteger(sample) && sample > 0);
}

function failure(key: string, code: string): Result<never> {
    return { ok: false, error: { key, code, severity: "error", args: [] } };
}

async function read<T>(
    signal: AbortSignal,
    decode: (value: unknown) => value is T,
    request?: Request,
): Promise<Result<T>> {
    const invalid: ApiError = { key: "ethernet_invalid_response", code: "ParseError", severity: "error", args: [] };
    let body: unknown;
    if (isTauri()) {
        try {
            body = request
                ? await invoke("ethernet_diagnostics", request)
                : await invoke("ethernet_interfaces");
        } catch (cause) {
            return {
                ok: false,
                error: toApiError(cause, { key: "ethernet_ipc_failure", code: "IOError", severity: "error", args: [] }),
            };
        }
    } else {
        let response: Response;
        const path = "/api/diagnostics/ethernet";
        try {
            response = request
                ? await fetch(path, {
                    method: "POST",
                    headers: { "Content-Type": "application/json", Accept: "application/json" },
                    body: JSON.stringify(request),
                    signal,
                })
                : await fetch(`${path}/interfaces`, { signal });
        } catch {
            return failure("ethernet_http_failure", "NetworkError");
        }
        try {
            body = await response.json();
        } catch {
            return { ok: false, error: invalid };
        }
        if (!response.ok) return { ok: false, error: toApiError(body, invalid) };
    }
    return decode(body) ? { ok: true, value: body, historyError: historySaveError(body) } : { ok: false, error: invalid };
}

export function listEthernetLinks(signal: AbortSignal): Promise<Result<EthernetLink[]>> {
    return read(signal, (value): value is EthernetLink[] =>
        Array.isArray(value) && value.every(isLink));
}

export function isEthernetReport(value: unknown): value is EthernetReport {
    return record(value) && isLink(value.link) && isProbeReport(value.probes) &&
        value.probes.interface === value.link.interface && value.probes.external.target !== null &&
        (value.hostname === undefined || (typeof value.hostname === "string" && value.hostname.length > 0));
}

export function ethernetTarget(report: EthernetReport): string | null {
    return report.hostname ?? report.probes.external.target;
}

export function runEthernet(
    interfaceName: string,
    target: string,
    signal: AbortSignal,
): Promise<Result<EthernetReport>> {
    return read(signal, (value): value is EthernetReport =>
        isEthernetReport(value) && value.link.interface === interfaceName &&
        ethernetTarget(value) === target,
    { interface: interfaceName, target });
}
