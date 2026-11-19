import { invoke, isTauri } from "@tauri-apps/api/core";
import { toApiError, type ApiError } from "../../i18n";
import { isConnectionReport, type ConnectionReport } from "../connection/connection-api";
import { isProbeReport, type ProbeReport } from "../probes/probe-api";
import { decodeRadio, type Radio } from "../radio/radio-api";

type Section<T> =
    | { status: "available"; data: T }
    | { status: "unavailable"; reason: string }
    | { status: "not_collected" };

type Hypothesis =
    | { key: "beyond_gateway"; gateway_avg_rtt_ms: number; external_avg_rtt_ms: number; started_at_ms: number; finished_at_ms: number }
    | { key: "sticky_client"; evidence: { candidate_bssid: string; minimum_signal_advantage_db: number; duration_ns: number } };

export type DiagnosticSession = {
    id: string;
    interface: string;
    platform: "linux" | "windows" | "android";
    started_at: { unix_time_ms: number | null };
    finished_at: { unix_time_ms: number | null };
    rules_version: number;
    radio: Section<Radio>;
    probes: Section<ProbeReport>;
    connection: Section<ConnectionReport>;
    hypotheses: Hypothesis[];
};

type Result = { ok: true; report: DiagnosticSession } | { ok: false; error: ApiError };

function record(value: unknown): value is Record<string, unknown> {
    return value !== null && typeof value === "object" && !Array.isArray(value);
}

function nonnegative(value: unknown): value is number {
    return typeof value === "number" && Number.isFinite(value) && value >= 0;
}

function date(value: unknown): value is number {
    return nonnegative(value) && value <= 8.64e15;
}

function timestamp(value: unknown): value is DiagnosticSession["started_at"] {
    return record(value) && (value.unix_time_ms === null || date(value.unix_time_ms));
}

function isHypothesis(value: unknown): value is Hypothesis {
    if (!record(value)) return false;
    if (value.key === "beyond_gateway") {
        return nonnegative(value.gateway_avg_rtt_ms) && nonnegative(value.external_avg_rtt_ms) &&
            date(value.started_at_ms) && date(value.finished_at_ms) && value.finished_at_ms >= value.started_at_ms;
    }
    return value.key === "sticky_client" && record(value.evidence) &&
        typeof value.evidence.candidate_bssid === "string" &&
        nonnegative(value.evidence.minimum_signal_advantage_db) && nonnegative(value.evidence.duration_ns);
}

function section<T>(value: unknown, decode: (value: unknown) => T | null): Section<T> | null {
    if (!record(value)) return null;
    if (value.status === "not_collected") return { status: "not_collected" };
    if (value.status === "unavailable" && typeof value.reason === "string") {
        return { status: "unavailable", reason: value.reason };
    }
    if (value.status !== "available") return null;
    const data = decode(value.data);
    return data === null ? null : { status: "available", data };
}

function decode(value: unknown, name: string, target: string | null): DiagnosticSession | null {
    if (!record(value) || typeof value.id !== "string" || value.interface !== name ||
        (value.platform !== "linux" && value.platform !== "windows" && value.platform !== "android") ||
        !timestamp(value.started_at) || !timestamp(value.finished_at) || typeof value.rules_version !== "number" ||
        !Array.isArray(value.hypotheses) || !value.hypotheses.every(isHypothesis)) return null;
    const radio = section(value.radio, (data) => decodeRadio(data, name));
    const probes = section(value.probes, (data) =>
        isProbeReport(data) && data.interface === name && data.external.target === target ? data : null);
    const connection = section(value.connection, (data) =>
        isConnectionReport(data) && data.interface === name ? data : null);
    if (!radio || !probes || !connection) return null;
    return {
        id: value.id, interface: name, platform: value.platform,
        started_at: value.started_at, finished_at: value.finished_at, rules_version: value.rules_version,
        radio, probes, connection, hypotheses: value.hypotheses,
    };
}

function error(key: string, code: string): ApiError {
    return { key, code, severity: "error", args: [] };
}

export async function runSession(interfaceName: string, target: string | null, signal: AbortSignal): Promise<Result> {
    let body: unknown;
    if (isTauri()) {
        try {
            body = await invoke<unknown>("diagnostic_session", { interface: interfaceName, target });
        } catch (cause) {
            return { ok: false, error: toApiError(cause, error("session_ipc_failure", "IOError")) };
        }
    } else {
        let response: Response;
        try {
            response = await fetch("/api/diagnostics/session", {
                method: "POST",
                headers: { "Content-Type": "application/json", Accept: "application/json" },
                body: JSON.stringify({ interface: interfaceName, target }),
                signal,
            });
        } catch {
            return { ok: false, error: error("session_http_failure", "NetworkError") };
        }
        try {
            body = await response.json();
        } catch {
            return { ok: false, error: error("session_invalid_response", "ParseError") };
        }
        if (!response.ok) return { ok: false, error: toApiError(body, error("session_invalid_response", "ParseError")) };
    }

    const report = decode(body, interfaceName, target);
    return report ? { ok: true, report } : { ok: false, error: error("session_invalid_response", "ParseError") };
}
