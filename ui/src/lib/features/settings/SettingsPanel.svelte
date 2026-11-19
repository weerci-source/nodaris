<script lang="ts">
    import { appearance, updateAppearance, type Language, type Theme } from "../../appearance.svelte";
    import { t, translateError, type ApiError } from "../../i18n";
    import Alert from "../../components/ui/Alert.svelte";
    import Button from "../../components/ui/Button.svelte";
    import Icon, { type IconName } from "../../components/ui/Icon.svelte";
    import TabList from "../../components/ui/TabList.svelte";
    import TabPanel from "../../components/ui/TabPanel.svelte";

    let { onOpenHistory }: { onOpenHistory: () => void } = $props();
    const id = $props.id();
    type SettingsTab = "appearance" | "data";
    let tab = $state<SettingsTab>("appearance");
    let saveError = $state<ApiError | null>(null);
    const languages: { value: Language; label: string }[] = [
        { value: "ru", label: "Русский" },
        { value: "en", label: "English" },
    ];
    const themes: { value: Theme; icon: IconName }[] = [
        { value: "system", icon: "monitor" },
        { value: "light", icon: "sun" },
        { value: "dark", icon: "moon" },
    ];
    let tabs = $derived([
        { value: "appearance" as const, label: t("settings_appearance") },
        { value: "data" as const, label: t("settings_data") },
    ]);

    function save(update: Parameters<typeof updateAppearance>[0]) {
        saveError = updateAppearance(update) ? null : {
            key: "settings_save_failed", code: "IOError", severity: "warning", args: [],
        };
    }
</script>

<section class="settings-screen" aria-labelledby={`${id}-title`}>
    <h1 class="workspace-title" id={`${id}-title`}>{t("settings_title")}</h1>
    <TabList group={id} label={t("settings_sections")} {tabs} active={tab} onSelect={(value) => { tab = value; }} />

    <TabPanel group={id} value="appearance" active={tab === "appearance"}>
        <div class="settings-content">
            {#if saveError}<Alert severity={saveError.severity}>{translateError(saveError)}</Alert>{/if}
            <fieldset class="card setting-card">
                <legend class="visually-hidden">{t("settings_language")}</legend>
                <div class="setting-label" aria-hidden="true">{t("settings_language")}</div>
                <div class="language-options">
                    {#each languages as option (option.value)}
                        <label class="preference-option">
                            <input type="radio" name={`${id}-language`} value={option.value}
                                checked={appearance.language === option.value}
                                onchange={() => save({ language: option.value })} />
                            <span lang={option.value}>{option.label}</span>
                        </label>
                    {/each}
                </div>
            </fieldset>

            <fieldset class="card setting-card setting-theme">
                <legend class="visually-hidden">{t("settings_theme")}</legend>
                <div class="setting-label" aria-hidden="true">{t("settings_theme")}</div>
                <div class="theme-options">
                    {#each themes as option (option.value)}
                        <label class="preference-option theme-option">
                            <input type="radio" name={`${id}-theme`} value={option.value}
                                checked={appearance.theme === option.value}
                                onchange={() => save({ theme: option.value })} />
                            <span class="theme-preview" data-preview={option.value} aria-hidden="true">
                                <span class="preview-window">
                                    <span class="preview-toolbar"><i></i><i></i><i></i></span>
                                    <span class="preview-body">
                                        <span class="preview-sidebar"></span>
                                        <span class="preview-content"><i></i><i></i><i></i></span>
                                    </span>
                                </span>
                            </span>
                            <span class="theme-caption">
                                <Icon name={option.icon} />
                                <span>{t(`settings_theme_${option.value}`)}</span>
                                {#if appearance.theme === option.value}
                                    <span class="theme-check"><Icon name="check" size={15} /></span>
                                {/if}
                            </span>
                        </label>
                    {/each}
                </div>
            </fieldset>
        </div>
    </TabPanel>

    <TabPanel group={id} value="data" active={tab === "data"}>
        <div class="settings-content">
            <section class="card ui-stack">
                <div>
                    <h2 class="setting-label ui-title">{t("settings_local_history")}</h2>
                    <p class="ui-muted">{t("history_retention")}</p>
                </div>
                <div><Button variant="secondary" onclick={onOpenHistory}>{t("settings_open_history")}</Button></div>
            </section>
        </div>
    </TabPanel>
</section>
