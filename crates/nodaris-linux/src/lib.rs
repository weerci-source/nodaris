use nodaris_core::{
    error::{CoreError, ErrorCode, ErrorSeverity},
    keys,
    models::AccessPoint,
};
use std::process::Command;

pub fn scan_wifi() -> Result<Vec<AccessPoint>, CoreError> {
    // Убрали флаг -s, так как он не поддерживается на MX Linux.
    // Используем стандартный terse-формат (-t), где разделитель — двоеточие.
    let output = Command::new("nmcli")
        .args(["-t", "-f", "SSID,BSSID,SIGNAL,FREQ", "dev", "wifi", "list"])
        .output()
        .map_err(|e| {
            CoreError::new(
                keys::scan::UNKNOWN_FAILURE,
                ErrorCode::HardwareError,
                ErrorSeverity::Error,
            )
            .with_tech(format!("Failed to execute nmcli: {}", e))
        })?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);

        // Проверяем, действительно ли это ошибка прав, или что-то другое
        let error_key =
            if err_msg.contains("Not authorized") || err_msg.contains("Permission denied") {
                keys::scan::WIFI_PERMISSION_DENIED
            } else {
                keys::scan::UNKNOWN_FAILURE
            };

        return Err(
            CoreError::new(error_key, ErrorCode::PermissionDenied, ErrorSeverity::Error)
                .with_tech(err_msg.to_string()),
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut aps = Vec::new();

    for line in stdout.lines() {
        // nmcli экранирует двоеточия внутри SSID и BSSID с помощью обратного слеша: \:
        // Пример строки: "My\:WiFi:AA\:BB\:CC\:DD\:EE\:FF:85:2412"
        // Мы временно заменим экранированные двоеточия на дефис
        let clean_line = line.replace("\\:", "-");

        // Теперь безопасно разбиваем по оставшимся (разделительным) двоеточиям
        let parts: Vec<&str> = clean_line.split(':').collect();

        if parts.len() >= 4 {
            // Возвращаем двоеточия обратно в SSID и BSSID
            let ssid = parts[0].replace("-", ":");
            let bssid = parts[1].replace("-", ":");

            let signal = parts[2].parse::<i32>().unwrap_or(0);
            let freq_str = parts[3].split_whitespace().next().unwrap_or("0");
            let frequency = freq_str.parse::<u32>().unwrap_or(0);

            if !ssid.is_empty() {
                aps.push(AccessPoint {
                    ssid,
                    bssid,
                    signal,
                    frequency,
                });
            }
        }
    }

    // Сортируем по уровню сигнала (от лучшего к худшему)
    aps.sort_by_key(|a| std::cmp::Reverse(a.signal));

    Ok(aps)
}
