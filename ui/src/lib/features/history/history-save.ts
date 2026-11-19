import { toApiError, type ApiError } from "../../i18n";

export function historySaveError(value: unknown): ApiError | null {
    if (!value || typeof value !== "object" || !("history_error" in value)) return null;
    return toApiError(value.history_error, {
        key: "history_save_failed", code: "IOError", severity: "warning", args: [],
    });
}
