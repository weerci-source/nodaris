<script lang="ts">
  import { onMount } from "svelte";
  import { translateError, type ApiError } from "./lib/i18n";

  type AccessPoint = {
    ssid: string;
    bssid: string;
    signal: number;
    frequency: number;
  };

  let aps = $state<AccessPoint[]>([]);
  let loading = $state(true);
  let error = $state<ApiError | null>(null);

  async function fetchScan() {
    loading = true;
    error = null;
    try {
      const res = await fetch("http://localhost:8080/api/scan");
      const data = await res.json();

      if (!res.ok) {
        // Сервер вернул структурированную ошибку
        error = data.error;
      } else {
        aps = data;
      }
    } catch (e: any) {
      // Если сервер недоступен (сетевая ошибка)
      error = {
        key: "scan_unknown_failure",
        code: "NetworkError",
        severity: "error",
        args: [],
      };
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    fetchScan();
  });
</script>

<main
  style="display: flex; flex-direction: column; height: 100vh; padding: var(--space-4);"
>
  <header
    style="display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--space-4);"
  >
    <h1 style="margin: 0;">
      Nodaris <span style="color: var(--color-primary); font-size: 0.6em;"
        >Network Scanner</span
      >
    </h1>
    <button class="btn" onclick={fetchScan} disabled={loading}>
      {loading ? "Scanning..." : "Scan Again"}
    </button>
  </header>

  {#if loading}
    <div class="card">Сканирование эфира...</div>
  {:else if error}
    <!-- Используем translateError для отображения -->
    <div
      class="card"
      style="border-color: var(--color-danger); color: var(--color-danger);"
    >
      {translateError(error)}
    </div>
  {:else if aps.length === 0}
    <div class="card">Сети не найдены. Убедитесь, что Wi-Fi включен.</div>
  {:else}
    <div class="card table-scroll" style="flex: 1; min-height: 0;">
      <table>
        <thead>
          <tr>
            <th>SSID</th>
            <th>BSSID (MAC)</th>
            <th>Сигнал (dBm)</th>
            <th>Частота (MHz)</th>
          </tr>
        </thead>
        <tbody>
          {#each aps as ap}
            <tr>
              <td><strong>{ap.ssid}</strong></td>
              <td style="color: var(--color-text-muted);">{ap.bssid}</td>
              <td>
                <span
                  style="color: {ap.signal > 50
                    ? 'var(--color-success)'
                    : ap.signal > 30
                      ? 'var(--color-warning)'
                      : 'var(--color-danger)'}"
                >
                  {ap.signal}
                </span>
              </td>
              <td>{ap.frequency}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</main>
