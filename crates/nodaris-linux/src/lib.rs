mod connection;
mod ethernet;
pub mod history;
mod icmp;
mod radio;
mod session;

pub use connection::monitor_connection;
pub use ethernet::{collect_ethernet_report, list_ethernet_links};
pub use icmp::collect_icmp_report;
pub use radio::collect_radio_report;
pub use radio::observe_connection;
pub use session::{collect_session, try_radio_slot};

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

#[derive(Debug, PartialEq, Eq)]
pub struct WifiInterface {
    pub name: String,
    pub connected: bool,
}

pub fn list_wifi_interfaces() -> Result<Vec<WifiInterface>, CoreError> {
    let output = Command::new("nmcli")
        .env("LC_ALL", "C")
        .args(["-t", "-f", "DEVICE,TYPE,STATE", "device", "status"])
        .output()
        .map_err(|error| {
            CoreError::hardware_err(keys::scan::UNKNOWN_FAILURE)
                .with_tech(format!("Failed to execute nmcli device status: {error}"))
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

    let stdout = std::str::from_utf8(&output.stdout).map_err(|error| {
        CoreError::hardware_err(keys::scan::UNKNOWN_FAILURE)
            .with_tech(format!("Invalid nmcli device status encoding: {error}"))
    })?;

    parse_wifi_interfaces(stdout)
}

fn parse_wifi_interfaces(stdout: &str) -> Result<Vec<WifiInterface>, CoreError> {
    let mut interfaces = Vec::new();

    for line in stdout.lines().filter(|line| !line.trim().is_empty()) {
        let fields = split_escaped(line, ':');
        let [name, device_type, state] = fields.as_slice() else {
            return Err(CoreError::hardware_err(keys::scan::UNKNOWN_FAILURE)
                .with_tech("Unexpected nmcli DEVICE,TYPE,STATE row format"));
        };

        if device_type == "wifi" {
            interfaces.push(WifiInterface {
                name: name.clone(),
                connected: matches!(state.as_str(), "connected" | "connected (externally)"),
            });
        }
    }

    Ok(interfaces)
}

#[cfg(test)]
mod interface_tests {
    use super::*;

    #[test]
    fn lists_wifi_interfaces_without_selecting_one() {
        let output = concat!(
            "eth0:ethernet:connected\n",
            "wlan0:wifi:connected\n",
            "wlan1:wifi:disconnected\n",
        );

        let interfaces = parse_wifi_interfaces(output).expect("valid device list");

        assert_eq!(
            interfaces,
            vec![
                WifiInterface {
                    name: "wlan0".into(),
                    connected: true,
                },
                WifiInterface {
                    name: "wlan1".into(),
                    connected: false,
                },
            ],
        );
    }

    #[test]
    fn ethernet_only_machine_has_no_wifi_interfaces() {
        let interfaces =
            parse_wifi_interfaces("eth0:ethernet:connected\n").expect("valid device list");

        assert!(interfaces.is_empty());
    }
}
