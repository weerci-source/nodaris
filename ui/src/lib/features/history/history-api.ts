import { invoke, isTauri } from "@tauri-apps/api/core";
import { toApiError, type ApiError } from "../../i18n";
import { isEthernetReport, type EthernetReport } from "../ethernet/ethernet-api";
import { decodeSession, type DiagnosticSession } from "../session/session-api";

export type HistoryEntry =
    | { id: string; kind: "session"; target: string | null; report: DiagnosticSession }
    | { id: string; kind: "ethernet"; target: string; report: EthernetReport };

type Result = { ok: true; entries: HistoryEntry[] } | { ok: false; error: ApiError };

function decodeEntry(value: unknown): HistoryEntry | null {
    if (!value || typeof value !== "object") return null;
    const { id, kind, target, report } = value as Record<string, unknown>;
    if (typeof id !== "string" || !/^linux-\d+-\d+-\d+$/.test(id)) return null;
    if (kind === "session" && (target === null || typeof target === "string")) {
        const session = decodeSession(report, target);
        return session?.id === id ? { id, kind, target, report: session } : null;
    }
    if (kind === "ethernet" && typeof target === "string" &&
        isEthernetReport(report) && report.probes.external.target === target) {
        return { id, kind, target, report };
    }
    return null;
}

function error(key: string, code: string): ApiError {
    return { key, code, severity: "error", args: [] };
}

export async function readHistory(signal: AbortSignal, deleteId?: string): Promise<Result> {
    let body: unknown;
    const invalid = error("history_invalid_response", "ParseError");
    if (isTauri()) {
        try {
            body = deleteId === undefined
                ? await invoke("diagnostic_history")
                : await invoke("delete_diagnostic_history", { id: deleteId });
        } catch (cause) {
            return { ok: false, error: toApiError(cause, error("history_ipc_failure", "IOError")) };
        }
    } else {
        const path = "/api/diagnostics/history";
        let response: Response;
        try {
            response = deleteId === undefined
                ? await fetch(path, { signal })
                : await fetch(`${path}/${encodeURIComponent(deleteId)}`, { method: "DELETE", signal });
        } catch {
            return { ok: false, error: error("history_http_failure", "NetworkError") };
        }
        try {
            body = await response.json();
        } catch {
            return { ok: false, error: invalid };
        }
        if (!response.ok) return { ok: false, error: toApiError(body, invalid) };
    }
    if (!Array.isArray(body)) return { ok: false, error: invalid };
    const entries: HistoryEntry[] = [];
    const ids = new Set<string>();
    for (const value of body) {
        const entry = decodeEntry(value);
        if (!entry || ids.has(entry.id)) return { ok: false, error: invalid };
        entries.push(entry);
        ids.add(entry.id);
    }
    return { ok: true, entries };
}
