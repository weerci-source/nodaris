export type Language = "ru" | "en";
export type Theme = "light" | "dark" | "system";

type Preferences = { language: Language; theme: Theme };
const storageKey = "nodaris.appearance";
const defaults: Preferences = { language: "ru", theme: "system" };

function load(): Preferences {
    try {
        const stored: unknown = JSON.parse(localStorage.getItem(storageKey) ?? "null");
        if (stored && typeof stored === "object") {
            const { language, theme } = stored as Partial<Preferences>;
            return {
                language: language === "en" || language === "ru" ? language : defaults.language,
                theme: theme === "light" || theme === "dark" || theme === "system" ? theme : defaults.theme,
            };
        }
    } catch {
        return { ...defaults };
    }
    return { ...defaults };
}

const state = $state(load());
const systemTheme = window.matchMedia("(prefers-color-scheme: dark)");

export const appearance = {
    get language() { return state.language; },
    get theme() { return state.theme; },
};

function apply() {
    document.documentElement.lang = state.language;
    document.documentElement.dataset.theme = state.theme === "system"
        ? (systemTheme.matches ? "dark" : "light")
        : state.theme;
}

export function initializeAppearance() {
    apply();
    systemTheme.addEventListener("change", apply);
}

export function updateAppearance(update: Partial<Preferences>): boolean {
    Object.assign(state, update);
    apply();
    try {
        localStorage.setItem(storageKey, JSON.stringify(state));
        return true;
    } catch {
        return false;
    }
}
