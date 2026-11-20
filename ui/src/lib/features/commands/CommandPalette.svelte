<script module lang="ts">
    import type { IconName } from "../../components/ui/Icon.svelte";

    export type Command<T extends string> = { id: T; label: string; keywords: string; icon?: IconName };
</script>

<script lang="ts" generics="T extends string">
    import { tick } from "svelte";
    import Alert from "../../components/ui/Alert.svelte";
    import Button from "../../components/ui/Button.svelte";
    import Icon from "../../components/ui/Icon.svelte";
    import LoadingStatus from "../../components/ui/LoadingStatus.svelte";
    import { t, translateError, type ApiError } from "../../i18n";

    let { commands, shortcut, onRun }: {
        commands: Command<T>[];
        shortcut: boolean;
        onRun: (id: T, dismiss: () => void, signal: AbortSignal) => Promise<ApiError | null>;
    } = $props();

    const id = $props.id();
    let dialog: HTMLDialogElement;
    let queryInput: HTMLInputElement;
    let scroll: HTMLDivElement;
    let query = $state("");
    let index = $state(0);
    let busy = $state(false);
    let error = $state<ApiError | null>(null);
    let active: AbortController | null = null;
    let opener: HTMLElement | null = null;

    function normalize(value: string) {
        return value.toLowerCase().normalize("NFD").replace(/\p{M}/gu, "");
    }

    let results = $derived.by(() => {
        const words = normalize(query).trim().split(/\s+/);
        return commands.filter((command) => {
            const text = normalize(`${command.label} ${command.keywords}`);
            return words.every((word) => text.includes(word));
        });
    });
    let selected = $derived(results[index]);

    function open(from?: HTMLElement) {
        if (dialog.open) return;
        opener = from ?? (document.activeElement instanceof HTMLElement ? document.activeElement : null);
        query = "";
        index = 0;
        error = null;
        dialog.showModal();
        scroll.scrollTop = 0;
        queryInput.focus();
    }

    function handleShortcut(event: KeyboardEvent) {
        if (!shortcut || !event.ctrlKey || event.metaKey || event.altKey || event.shiftKey ||
            event.isComposing || (event.key.toLowerCase() !== "k" && event.code !== "KeyK")) return;
        event.preventDefault();
        open();
    }

    function dismiss() {
        active?.abort();
        active = null;
        busy = false;
        dialog.close();
        opener?.focus();
        opener = null;
    }

    function handleCancel(event: Event) {
        event.preventDefault();
        dismiss();
    }

    async function run(command: Command<T>) {
        if (active) return;
        const request = new AbortController();
        active = request;
        busy = true;
        error = null;
        const result = await onRun(command.id, dismiss, request.signal);
        if (request.signal.aborted) return;
        active = null;
        busy = false;
        error = result;
    }

    function resetSearch() {
        index = 0;
        error = null;
        scroll.scrollTop = 0;
    }

    function navigate(event: KeyboardEvent) {
        if (event.isComposing) return;
        if (event.key === "Escape") {
            event.preventDefault();
            event.stopPropagation();
            dismiss();
            return;
        }
        if (busy || !results.length ||
            !["ArrowDown", "ArrowUp", "Enter"].includes(event.key)) return;
        event.preventDefault();
        if (event.key === "Enter") {
            if (selected && !event.repeat) void run(selected);
            return;
        }
        const step = event.key === "ArrowDown" ? 1 : -1;
        index = (index + step + results.length) % results.length;
        document.getElementById(`${id}-option-${results[index].id}`)?.scrollIntoView({ block: "nearest" });
    }
</script>

<svelte:window onkeydown={handleShortcut} />

<Button variant="secondary" size="sm" icon="search" class="command-trigger"
    aria-haspopup="dialog" aria-controls={id} aria-keyshortcuts={shortcut ? "Control+K" : undefined}
    onclick={(event) => open(event.currentTarget)}>
    {t("commands_title")}
    {#if shortcut}<kbd aria-hidden="true">Ctrl K</kbd>{/if}
</Button>

<dialog class="command-dialog" {id} aria-label={t("commands_title")} bind:this={dialog} oncancel={handleCancel}>
    <div class="command-search">
        <Icon name="search" size={20} />
        <input class="command-input" type="search" role="combobox"
            aria-label={t("commands_search")} placeholder={t("commands_search")}
            aria-autocomplete="list" aria-expanded="true" aria-controls={`${id}-results`}
            aria-activedescendant={selected ? `${id}-option-${selected.id}` : undefined}
            readonly={busy} bind:this={queryInput} bind:value={query} oninput={resetSearch}
            onkeydown={navigate} />
        <Button variant="plain" size="sm" icon="close" aria-label={t("commands_close")} onclick={dismiss}>{""}</Button>
    </div>
    {#if error}
        <div class="command-message">
            <Alert severity={error.severity}>
                {translateError(error)}
                {#if error.request_id}<p class="ui-muted">{t("session_request_id", [error.request_id])}</p>{/if}
            </Alert>
        </div>
    {/if}
    <div class="command-scroll" bind:this={scroll}>
        <ul class="command-list" id={`${id}-results`} role="listbox" aria-label={t("commands_title")} aria-busy={busy}>
            {#each results as command, position (command.id)}
                <li role="presentation">
                    <button class="command-option" type="button" role="option" tabindex="-1" id={`${id}-option-${command.id}`}
                        aria-selected={index === position} disabled={busy}
                        onpointerenter={() => { if (!busy) index = position; }}
                        onmousedown={(event) => event.preventDefault()} onclick={() => run(command)}>
                        {#if command.icon}<span class="command-option-icon"><Icon name={command.icon} /></span>{/if}
                        <span>{command.label}</span>
                    </button>
                </li>
            {/each}
        </ul>
        {#if !results.length}<p class="command-message ui-muted" role="status">{t("commands_empty")}</p>{/if}
    </div>
    {#if busy}
        <div class="command-message"><LoadingStatus message={t("history_loading")} /></div>
    {:else}
        <footer class="command-footer">
            <span><kbd>↑</kbd><kbd>↓</kbd> {t("commands_choose")}</span>
            <span><kbd>Enter</kbd> {t("commands_run")}</span>
            <span><kbd>Esc</kbd> {t("commands_close")}</span>
        </footer>
    {/if}
</dialog>
