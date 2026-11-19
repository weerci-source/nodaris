import ru from "../locales/ru.json";
import en from "../locales/en.json";

const dictionaries: Record<string, Record<string, string>> = { ru, en };
export const currentLang = "ru";

export type ApiError = {
    key: string;
    code: string;
    severity: "info" | "warning" | "error" | "critical";
    args: string[];
    request_id?: string;
};

function asRecord(value: unknown): Record<string, unknown> | null {
    return typeof value === "object" && value !== null && !Array.isArray(value)
        ? value as Record<string, unknown>
        : null;
}

export function toApiError(
    value: unknown,
    fallback: ApiError = { key: "scan_unknown_failure", code: "InternalError", severity: "error", args: [] },
): ApiError {
    const root = asRecord(value);
    const error = root?.ok === false ? asRecord(root.error) : root;
    if (error) {
        const { key, code, severity, args } = error;
        const requestId = error.request_id ?? root?.request_id;
        if (
            typeof key === "string" && key.length > 0 &&
            typeof code === "string" && code.length > 0 &&
            (severity === "info" || severity === "warning" || severity === "error" || severity === "critical") &&
            (args === undefined || (Array.isArray(args) && args.every((arg) => typeof arg === "string")))
        ) {
            return {
                key, code, severity, args: args ?? [],
                ...(typeof requestId === "string" ? { request_id: requestId } : {}),
            };
        }
    }
    return fallback;
}

export function t(key: string, args: string[] = []): string {
    const dict = dictionaries[currentLang];
    const fallback = dictionaries.en;
    const template = Object.hasOwn(dict, key)
        ? dict[key]
        : Object.hasOwn(fallback, key) ? fallback[key] : key;
    return template.replace(/\{(\d+)\}/g, (match, index: string) => args[Number(index)] ?? match);
}

export function translateError(error: ApiError): string {
    const translated = t(error.key, error.args);
    return translated !== error.key ? translated : t("unknown_error");
}
