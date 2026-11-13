use nodaris_core::{
    error::{CoreError, ErrorCode, ErrorSeverity},
    keys,
    models::AccessPoint,
};
use std::fs;
use std::process::Command;

// Надежный парсер, который не ломается на двоеточиях в SSID и BSSID
fn split_escaped(input: &str, delim: char) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&next) = chars.peek() {
                if next == delim {
                    current.push(next);
                    chars.next();
                    continue;
                }
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
        .args(["-t", "-f", "IN-USE,SSID,BSSID,SIGNAL,FREQ", "dev", "wifi", "list"])
        .output()
        .map_err(|e| {
            CoreError::new(keys::scan::UNKNOWN_FAILURE, ErrorCode::HardwareError, ErrorSeverity::Error)
                .with_tech(format!("Failed to execute nmcli: {}", e))
        })?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        let error_key = if err_msg.contains("Not authorized") || err_msg.contains("Permission denied") {
            keys::scan::WIFI_PERMISSION_DENIED
        } else {
            keys::scan::UNKNOWN_FAILURE
        };
        return Err(CoreError::new(error_key, ErrorCode::PermissionDenied, ErrorSeverity::Error)
            .with_tech(err_msg.to_string()));
    }

    // Получаем реальный сигнал и шум для активного интерфейса из /proc/net/wireless
    let mut real_sig = None;
    let mut real_noise = None;

    if let Ok(out) = Command::new("nmcli").args(["-t", "-f", "DEVICE,TYPE", "device"]).output() {
        let s = String::from_utf8_lossy(&out.stdout);
        if let Some(iface) = s.lines().find(|l| l.contains(":wifi")).map(|l| l.split(':').next().unwrap_or("")) {
            if let Ok(content) = fs::read_to_string("/proc/net/wireless") {
                for line in content.lines().skip(2) {
                    if line.trim().starts_with(&format!("{}:", iface)) {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 5 {
                            let sig_str = parts[3].trim_end_matches('.');
                            let noise_str = parts[4].trim_end_matches('.');
                            if let (Ok(sig), Ok(noise)) = (sig_str.parse::<i32>(), noise_str.parse::<i32>()) {
                                real_sig = Some(sig);
                                // Отбрасываем мусор (0 или -256)
                                real_noise = if noise < 0 && noise > -120 { Some(noise) } else { None };
                            }
                        }
                    }
                }
            }
        }
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut aps = Vec::new();

    for line in stdout.lines() {
        let parts = split_escaped(line, ':');
        
        // Теперь у нас 5 частей: IN-USE, SSID, BSSID, SIGNAL, FREQ
        if parts.len() >= 5 {
            let in_use = parts[0].trim() == "*" || parts[0].trim() == "yes";
            let ssid = parts[1].clone(); // Дефисы в названиях больше не ломаются!
            let bssid = parts[2].clone();
            let signal = parts[3].parse::<i32>().unwrap_or(0);
            let frequency = parts[4].split_whitespace().next().unwrap_or("0").parse::<u32>().unwrap_or(0);

            if !ssid.is_empty() {
                let mut noise = None;
                let mut snr = None;
                let mut is_active = false;

                if in_use {
                    is_active = true;
                    noise = real_noise;
                    // SNR считаем только если драйвер отдал адекватный шум
                    if let (Some(s), Some(n)) = (real_sig, real_noise) {
                        snr = Some(s - n);
                    }
                }

                aps.push(AccessPoint {
                    ssid, bssid, signal, frequency, noise, snr, is_active,
                });
            }
        }
    }

    aps.sort_by_key(|a| std::cmp::Reverse(a.signal));
    Ok(aps)
}