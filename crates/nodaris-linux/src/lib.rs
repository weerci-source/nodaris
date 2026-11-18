use nodaris_core::{error::CoreError, keys, models::access_point::AccessPoint};
use std::process::Command;

// Надежный парсер, который не ломается на двоеточиях в SSID и BSSID
fn split_escaped(input: &str, delim: char) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&next) = chars.peek()
                && next == delim
            {
                current.push(next);
                chars.next();
                continue;
            }
            current.push(c);
        } else if c == delim {
            result.push(current);
            current = String::new();
        } else {
            current.push(c);
        }
    }
    result.push(current);
    result
}

pub fn scan_wifi() -> Result<Vec<AccessPoint>, CoreError> {
    // Запрашиваем IN-USE (активна ли сеть) прямо в списке сетей
    let output = Command::new("nmcli")
        .args([
            "-t",
            "-f",
            "IN-USE,SSID,BSSID,SIGNAL,FREQ",
            "dev",
            "wifi",
            "list",
        ])
        .output()
        .map_err(|e| {
            CoreError::hardware_err(keys::scan::UNKNOWN_FAILURE)
                .with_tech(format!("Failed to execute nmcli: {}", e))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        let error = if stderr.contains("Not authorized") || stderr.contains("Permission denied") {
            CoreError::permission_err(keys::scan::WIFI_PERMISSION_DENIED)
        } else {
            CoreError::hardware_err(keys::scan::UNKNOWN_FAILURE)
        };

        return Err(error.with_tech(stderr.into_owned()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut aps = Vec::new();

    for line in stdout.lines() {
        let parts = split_escaped(line, ':');
        if parts.len() < 5 || parts[1].is_empty() {
            continue;
        }

        let signal_quality_percent = parts[3]
            .trim()
            .parse::<u8>()
            .ok()
            .filter(|&value| value <= 100);

        let Some(frequency_mhz) = parts[4]
            .split_whitespace()
            .next()
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|&value| value > 0)
        else {
            // В текущей модели частота обязательна.
            // Не подставляем 0 МГц вместо невалидного значения.
            continue;
        };

        aps.push(AccessPoint {
            ssid: parts[1].clone(),
            bssid: parts[2].clone(),
            rssi_dbm: None,
            signal_quality_percent,
            frequency_mhz,
            channel: None,
            channel_width_mhz: None,
            is_connected: matches!(parts[0].trim(), "*" | "yes"),
        });
    }

    aps.sort_by_key(|ap| std::cmp::Reverse(ap.signal_quality_percent));

    Ok(aps)
}
