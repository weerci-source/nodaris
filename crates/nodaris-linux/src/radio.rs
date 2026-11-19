use std::{io::ErrorKind, process::Command};

use nodaris_core::models::{
    diagnostic_report::{DiagnosticReport, ReportSection},
    measurement::{Measurement, ObservationTime, SourceId, UnavailableReason},
    radio::SignalQualityPercent,
    radio_observation::{Bssid, FrequencyMhz, InterfaceId, RadioObservation},
};

use super::split_escaped;

struct ConnectedAp {
    bssid: Bssid,
    frequency: Option<FrequencyMhz>,
    quality: Option<SignalQualityPercent>,
}

/// Читает радиоданные выбранного интерфейса из NetworkManager.
/// read_time вызывается после получения данных и отмечает момент чтения.
pub fn collect_radio_report(
    interface: &InterfaceId,
    read_time: impl FnOnce() -> ObservationTime,
    max_snr_time_difference_ns: u64,
) -> DiagnosticReport {
    let radio = match read_connected_ap(interface) {
        Ok(ap) => ReportSection::Available(to_observation(interface, ap, read_time())),
        Err(reason) => ReportSection::Unavailable(reason),
    };

    DiagnosticReport::from_radio(radio, max_snr_time_difference_ns)
}

fn read_connected_ap(interface: &InterfaceId) -> Result<ConnectedAp, UnavailableReason> {
    if interface.0.is_empty() {
        return Err(UnavailableReason::NotConfigured);
    }

    let output = Command::new("nmcli")
        .env("LC_ALL", "C")
        .args([
            "-t",
            "-f",
            "IN-USE,BSSID,SIGNAL,FREQ",
            "device",
            "wifi",
            "list",
            "ifname",
        ])
        .arg(&interface.0)
        .args(["--rescan", "no"])
        .output()
        .map_err(|error| match error.kind() {
            ErrorKind::NotFound => UnavailableReason::NotConfigured,
            ErrorKind::PermissionDenied => UnavailableReason::PermissionDenied,
            _ => UnavailableReason::TemporarilyUnavailable,
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
        return Err(
            if stderr.contains("not authorized") || stderr.contains("permission denied") {
                UnavailableReason::PermissionDenied
            } else if output.status.code() == Some(10) {
                UnavailableReason::HardwareUnavailable
            } else {
                UnavailableReason::TemporarilyUnavailable
            },
        );
    }

    let stdout =
        std::str::from_utf8(&output.stdout).map_err(|_| UnavailableReason::InvalidMeasurement)?;

    parse_connected_ap(stdout)
}

fn parse_connected_ap(stdout: &str) -> Result<ConnectedAp, UnavailableReason> {
    let mut connected = None;

    for line in stdout.lines() {
        let parts = split_escaped(line, ':');
        if !matches!(parts.first().map(|field| field.trim()), Some("*" | "yes")) {
            continue;
        }

        let [_, bssid, quality, frequency] = parts.as_slice() else {
            return Err(UnavailableReason::InvalidMeasurement);
        };

        let bssid = bssid.trim();
        let octets = bssid.split(':');
        if connected.is_some()
            || octets.clone().count() != 6
            || !octets
                .into_iter()
                .all(|octet| octet.len() == 2 && octet.bytes().all(|byte| byte.is_ascii_hexdigit()))
        {
            return Err(UnavailableReason::InvalidMeasurement);
        }

        connected = Some(ConnectedAp {
            bssid: Bssid(bssid.to_ascii_uppercase()),
            frequency: frequency
                .split_whitespace()
                .next()
                .and_then(|value| value.parse::<u32>().ok())
                .filter(|&value| value > 0)
                .map(FrequencyMhz),
            quality: quality
                .trim()
                .parse::<u8>()
                .ok()
                .and_then(|value| SignalQualityPercent::try_from(value).ok()),
        });
    }

    // Список прочитан, но в нём нет активной точки.
    // Это не доказательство того, что Wi-Fi выключен.
    connected.ok_or(UnavailableReason::NotMeasured)
}

fn to_observation(
    interface: &InterfaceId,
    ap: ConnectedAp,
    observed_at: ObservationTime,
) -> RadioObservation {
    let source = SourceId("linux.network_manager".into());

    let signal_quality = match ap.quality {
        Some(value) => Measurement::available(value, source.clone(), observed_at.clone()),
        None => Measurement::unavailable(
            UnavailableReason::InvalidMeasurement,
            source.clone(),
            observed_at.clone(),
        ),
    };

    RadioObservation {
        interface: interface.clone(),
        bssid: Some(ap.bssid),
        frequency: ap.frequency,
        rssi: Some(Measurement::unavailable(
            UnavailableReason::NotSupported,
            source.clone(),
            observed_at.clone(),
        )),
        noise: Some(Measurement::unavailable(
            UnavailableReason::NotSupported,
            source,
            observed_at,
        )),
        signal_quality: Some(signal_quality),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_active_ap_even_when_an_inactive_ap_is_stronger() {
        let output = concat!(
            ":02\\:00\\:00\\:00\\:00\\:01:91:2412 MHz\n",
            "*:02\\:00\\:00\\:00\\:00\\:02:58:5260 MHz\n",
        );

        let ap = parse_connected_ap(output).expect("active AP");

        assert_eq!(ap.bssid, Bssid("02:00:00:00:00:02".into()));
        assert_eq!(ap.frequency, Some(FrequencyMhz(5260)));
        assert_eq!(ap.quality.map(SignalQualityPercent::value), Some(58));
    }

    #[test]
    fn does_not_pick_an_inactive_ap_when_no_active_ap_is_reported() {
        let output = ":02\\:00\\:00\\:00\\:00\\:01:91:2412 MHz\n";

        assert!(matches!(
            parse_connected_ap(output),
            Err(UnavailableReason::NotMeasured),
        ));
    }

    #[test]
    fn keeps_the_ap_when_its_quality_value_is_invalid() {
        let output = "*:02\\:00\\:00\\:00\\:00\\:02:101:5260 MHz\n";

        let ap = parse_connected_ap(output).expect("active AP");

        assert_eq!(ap.frequency, Some(FrequencyMhz(5260)));
        assert_eq!(ap.quality, None);
    }
}
