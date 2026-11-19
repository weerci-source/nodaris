import { toApiError, type ApiError } from "../../i18n";

type Route = { interface: string; gateway: string | null; source?: string | null; table?: string | null };
type Statistics = {
    sent: number;
    received: number;
    min_rtt_ms: number | null;
    avg_rtt_ms: number | null;
    max_rtt_ms: number | null;
    jitter_ms: number | null;
    packet_loss_percent: number;
};
type Attempt = {
    started_offset_ms: number;
    finished_offset_ms: number;
    outcome: { status: "reply"; rtt_ms: number } |
        { status: "no_reply" | "icmp_error" | "unavailable" };
};
type Series = {
    target: string | null;
    route: Route | null;
    started_offset_ms: number;
    finished_offset_ms: number;
    statistics: Statistics | null;
    attempts?: Attempt[];
    unavailable_reason: string | null;
};
export type ProbeReport = {
    interface: string;
    method: "icmp";
    started_at_unix_ms: number;
    gateway: Series;
    external: Series;
};
type Result = { ok: true; report: ProbeReport } | { ok: false; error: ApiError };

function record(value: unknown): value is Record<string, unknown> {
    return value !== null && typeof value === "object" && !Array.isArray(value);
}

function optionalText(value: unknown): value is string | null {
    return value === null || typeof value === "string";
}

function nonnegative(value: unknown): value is number {
    return typeof value === "number" && Number.isFinite(value) && value >= 0;
}

function isStatistics(value: unknown): value is Statistics {
    return record(value) && nonnegative(value.sent) && nonnegative(value.received) &&
        nonnegative(value.packet_loss_percent) && value.packet_loss_percent <= 100 &&
        [value.min_rtt_ms, value.avg_rtt_ms, value.max_rtt_ms, value.jitter_ms]
            .every((metric) => metric === null || nonnegative(metric));
}

function isAttempt(value: unknown): value is Attempt {
    if (!record(value) || !nonnegative(value.started_offset_ms) || !nonnegative(value.finished_offset_ms) ||
        value.finished_offset_ms < value.started_offset_ms || !record(value.outcome)) return false;
    switch (value.outcome.status) {
        case "reply": return nonnegative(value.outcome.rtt_ms);
        case "no_reply":
        case "icmp_error":
        case "unavailable": return true;
        default: return false;
    }
}

function isSeries(value: unknown): value is Series {
    return record(value) && optionalText(value.target) && optionalText(value.unavailable_reason) &&
        nonnegative(value.started_offset_ms) && nonnegative(value.finished_offset_ms) &&
        value.finished_offset_ms >= value.started_offset_ms &&
        (value.attempts === undefined || (Array.isArray(value.attempts) && value.attempts.every(isAttempt))) &&
        (value.statistics === null || isStatistics(value.statistics)) &&
        (value.route === null || (
            record(value.route) && typeof value.route.interface === "string" && optionalText(value.route.gateway) &&
            (value.route.source === undefined || optionalText(value.route.source)) &&
            (value.route.table === undefined || optionalText(value.route.table))
        ));
}

export function isProbeReport(value: unknown): value is ProbeReport {
    return record(value) && value.method === "icmp" && typeof value.interface === "string" &&
        nonnegative(value.started_at_unix_ms) && value.started_at_unix_ms <= 8.64e15 &&
        isSeries(value.gateway) && isSeries(value.external);
}

export function probeError(key: string, code: string): ApiError {
    return { key, code, severity: "error", args: [] };
}

export async function collectProbes(
    wifiInterface: string,
    target: string,
    signal: AbortSignal,
): Promise<Result> {
    let response: Response;
    try {
        response = await fetch("/api/diagnostics/probes", {
            method: "POST",
            headers: { "Content-Type": "application/json", Accept: "application/json" },
            body: JSON.stringify({ interface: wifiInterface, target }),
            signal,
        });
    } catch {
        return { ok: false, error: probeError("probe_transport_failure", "NetworkError") };
    }
    try {
        const value: unknown = await response.json();
        if (!response.ok && record(value) && value.ok === false) {
            return { ok: false, error: toApiError(value) };
        }
        if (response.ok && isProbeReport(value) && value.interface === wifiInterface && value.external.target === target) {
            return { ok: true, report: value };
        }
    } catch {
        // Responses without the expected JSON contract share the same localized error.
    }
    return { ok: false, error: probeError("probe_invalid_response", "ParseError") };
}
