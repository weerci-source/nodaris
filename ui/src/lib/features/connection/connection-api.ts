import { toApiError, type ApiError } from "../../i18n";

type TransitionKind = "access_point_changed" | "network_changed" | "connected" | "disconnected";
export type ConnectionEndpoint = {
    observed_at: { unix_time_ms: number | null };
    bssid: string | null;
    frequency_mhz: number | null;
    rssi_dbm: number | null;
};
type Transition = {
    kind: TransitionKind;
    detection_method: "snapshot_comparison" | "operating_system_event";
    source: string;
    before: ConnectionEndpoint;
    after: ConnectionEndpoint;
};
type StickyAssessment =
    | "NoEvidence"
    | { Suspected: Record<string, unknown> }
    | { InsufficientData: string | { MeasurementUnavailable: string } };

export type ConnectionReport = {
    interface: string;
    elapsed_ms: number;
    sample_count: number;
    unavailable_count: number;
    transitions: Transition[];
    sticky_client: { assessment: StickyAssessment };
};

type Result = { ok: true; report: ConnectionReport } | { ok: false; error: ApiError };

function record(value: unknown): value is Record<string, unknown> {
    return value !== null && typeof value === "object" && !Array.isArray(value);
}

function nonnegativeInteger(value: unknown): value is number {
    return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}

function isEndpoint(value: unknown): value is ConnectionEndpoint {
    if (!record(value) || !record(value.observed_at)) return false;
    const timestamp = value.observed_at.unix_time_ms;
    return (timestamp === null || (nonnegativeInteger(timestamp) && timestamp <= 8.64e15)) &&
        (value.bssid === null || typeof value.bssid === "string") &&
        (value.frequency_mhz === null || (nonnegativeInteger(value.frequency_mhz) && value.frequency_mhz > 0)) &&
        (value.rssi_dbm === null || (typeof value.rssi_dbm === "number" &&
            Number.isSafeInteger(value.rssi_dbm) && value.rssi_dbm < 0));
}

function isTransition(value: unknown): value is Transition {
    return record(value) &&
        (value.kind === "access_point_changed" || value.kind === "network_changed" ||
            value.kind === "connected" || value.kind === "disconnected") &&
        (value.detection_method === "snapshot_comparison" || value.detection_method === "operating_system_event") &&
        typeof value.source === "string" && isEndpoint(value.before) && isEndpoint(value.after);
}

function isAssessment(value: unknown): value is StickyAssessment {
    if (value === "NoEvidence") return true;
    if (!record(value)) return false;
    return record(value.Suspected) || typeof value.InsufficientData === "string" ||
        (record(value.InsufficientData) && typeof value.InsufficientData.MeasurementUnavailable === "string");
}

export function isConnectionReport(value: unknown): value is ConnectionReport {
    return record(value) && typeof value.interface === "string" &&
        nonnegativeInteger(value.elapsed_ms) && nonnegativeInteger(value.sample_count) &&
        nonnegativeInteger(value.unavailable_count) && value.unavailable_count <= value.sample_count &&
        Array.isArray(value.transitions) && value.transitions.every(isTransition) &&
        record(value.sticky_client) && isAssessment(value.sticky_client.assessment);
}

function failure(key: string, code: string): Result {
    return { ok: false, error: { key, code, severity: "error", args: [] } };
}

export async function collectConnection(interfaceName: string, signal: AbortSignal): Promise<Result> {
    let response: Response;
    try {
        response = await fetch("/api/diagnostics/connection", {
            method: "POST",
            headers: { "Content-Type": "application/json", Accept: "application/json" },
            body: JSON.stringify({ interface: interfaceName }),
            signal,
        });
    } catch {
        return failure("connection_transport_failure", "NetworkError");
    }

    try {
        const body: unknown = await response.json();
        if (!response.ok && record(body) && body.ok === false) {
            return { ok: false, error: toApiError(body) };
        }
        if (response.ok && isConnectionReport(body) && body.interface === interfaceName) {
            return { ok: true, report: body };
        }
    } catch {
        return failure("connection_invalid_response", "ParseError");
    }
    return failure("connection_invalid_response", "ParseError");
}
