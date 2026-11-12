# Обработка ошибок в Nodaris

Документ описывает единый сквозной контракт обработки ошибок: от `CoreError` в Rust до локализованного сообщения в UI.

Правила обязательны для всего кода проекта: Rust, TypeScript, Svelte, API/transport layer, локализации.

Главный принцип:
> **Ошибка должна оставаться структурой до самого последнего этапа отображения пользователю.**
> Строка является представлением ошибки, а не самой ошибкой.

## 0. TL;DR
1. Никогда не передавайте ошибку как голую строку, если она может быть структурой.
2. У каждой пользовательской ошибки должен быть стабильный `key` (идентификатор локализации).
3. `code` — машинная категория, `key` — конкретная причина.
4. `args` передаются отдельными значениями, не склеиваются через `format!`.
5. Текст формируется только на клиенте через `key` + `args`.
6. Внутренние ошибки и stack trace не попадают в пользовательское сообщение (только в логи).
7. Сетевые ошибки клиента (Tauri IPC, WebSocket) и HTTP-ошибки сервера — разные классы.
8. Для диагностики используется `request_id` / `correlation_id`.

## 1. Архитектура
```text
core-models::CoreError (key, code, severity, args, tech_msg)
  ↓
core-api::ApiError (key, code, severity, args, request_id)
  ↓ (JSON)
client/core/transport.ts (ApiCallError extends Error)
  ↓
translateError(e.api) + i18n.t(e.api.key, e.api.args)
  ↓
Локализованная строка в UI
```

## 2. Контракт ошибки
### 2.1. Поля
* `key`: Стабильный идентификатор (например, `scan_wifi_permission_denied`).
* `code`: Категория (`ErrorCode`):
  * `NetworkError` — проблемы с сетью (таймауты, отключение интерфейса).
  * `PermissionDenied` — нет прав (например, нет `CAP_NET_ADMIN` для сканирования).
  * `HardwareError` — сетевая карта не отвечает.
  * `ValidationError` — пользовательская валидация.
  * `ParseError` — ошибка разбора пакетов/данных.
  * `InternalError` — неизвестная причина.
  * `IOError` — файловые операции.
* `severity`: `info`, `warning`, `error`, `critical`. Влияет на UI (цвет, иконка).
* `args`: Массив значений для подстановки в `{0}`, `{1}` и т.д.
* `request_id`: Для поиска в логах.

## 3. Безопасность пользовательских данных
В `args` **запрещено** помещать:
* stack trace, SQL, сырые MAC-адреса оборудования (если это не UI-специфика), пути файловой системы.
* внутренние ошибки драйверов (например, `nl80211` errno).
Все технические детали уходят в `technical_message` (только для логов).

## 4. Rust: Конструкторы `CoreError`
Используется паттерн Builder.
* `CoreError::validation(key)` + `.with_args()`
* `CoreError::network_err(key)` + `.with_tech()`
* `CoreError::hardware_err(key)`
* `CoreError::internal(msg)` (для критических)
* `CoreError::internal_warning(msg)`

## 5. Каталог ключей `keys.rs`
Все ключи хранятся в `crates/nodaris-core/src/keys.rs`.
```rust
pub mod scan {
    pub const WIFI_PERMISSION_DENIED: &str = "scan_wifi_permission_denied";
    pub const INTERFACE_DOWN: &str = "scan_interface_down";
}
pub mod api {
    pub const MALFORMED_REQUEST: &str = "api_malformed_request";
}
```

## 6. TypeScript / Svelte
Никогда не делайте `e.message`. Используйте `toApiError(e)`.
В Svelte 5 ошибки хранятся в состоянии:
```svelte
<script lang="ts">
  let error = $state<ApiError | null>(null);
</script>

{#if error}
  <div class="alert" data-severity={error.severity}>
    {translateError(error)}
  </div>
{/if}
```

## 7. Чек-лист перед PR
* [ ] Нет `from_key` в бизнес-логике.
* [ ] Аргументы не склеены через `format!`.
* [ ] Технические детали (errno, path) — в `.with_tech()`.
* [ ] Новые ключи добавлены в `ru.json` и `en.json`.
* [ ] В Svelte используется `translateError()`.