import { invoke, isTauri } from "@tauri-apps/api/core";
import { toApiError, type ApiError } from "../../i18n";
import { decodeScan, isRecord, type AccessPoint } from "./scan-model";

type ScanResult = { ok: true; points: AccessPoint[] } | { ok: false; error: ApiError };

function failure(key: string, code: string): ScanResult {
    return { ok: false, error: { key, code, severity: "error", args: [] } };
}

export async function fetchScan(signal: AbortSignal): Promise<ScanResult> {
    let body: unknown;
    let httpError = false;

    if (isTauri()) {
        try {
            body = await invoke<unknown>("scan_wifi");
        } catch (error: unknown) {
            return { ok: false, error: toApiError(error, {
                key: "session_ipc_failure", code: "IOError", severity: "error", args: [],
            }) };
        }
    } else {
        let response: Response;
        try {
            response = await fetch("/api/scan", { signal, headers: { Accept: "application/json" }, cache: "no-store" });
        } catch {
            return failure("scan_transport_failure", "NetworkError");
        }

        httpError = !response.ok;
        try {
            body = await response.json();
        } catch {
            return httpError
                ? failure("scan_unknown_failure", "InternalError")
                : failure("scan_invalid_response", "ParseError");
        }
    }

    if (httpError || (isRecord(body) && body.ok === false)) {
        return { ok: false, error: toApiError(body) };
    }

    const points = decodeScan(body);
    return points === null
        ? failure("scan_invalid_response", "ParseError")
        : { ok: true, points };
}
