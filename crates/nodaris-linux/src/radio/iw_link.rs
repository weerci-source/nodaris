use std::{io::ErrorKind, path::Path, process::Command};

use nodaris_core::models::measurement::UnavailableReason;

pub(super) struct IwLink {
    pub(super) bssid: String,
    pub(super) frequency_mhz: u32,
    pub(super) rssi_dbm: i16,
}

pub(super) fn read_link(interface: &str) -> Result<IwLink, UnavailableReason> {
    let iw = ["/usr/sbin/iw", "/sbin/iw"]
        .into_iter()
        .find(|path| Path::new(path).is_file())
        .unwrap_or("iw");
    // Ограничиваем время жизни самой команды, а не только ожидание HTTP-ответа.
    let output = Command::new("timeout")
        .env("LC_ALL", "C")
        .args(["--kill-after=1s", "2s", iw, "dev", interface, "link"])
        .output()
        .map_err(|error| match error.kind() {
            ErrorKind::NotFound => UnavailableReason::NotSupported,
            ErrorKind::PermissionDenied => UnavailableReason::PermissionDenied,
            _ => UnavailableReason::TemporarilyUnavailable,
        })?;

    match output.status.code() {
        Some(0) => {}
        Some(127) => return Err(UnavailableReason::NotSupported),
        Some(126) => return Err(UnavailableReason::PermissionDenied),
        _ => return Err(UnavailableReason::TemporarilyUnavailable),
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_link(&stdout)
}

fn parse_link(output: &str) -> Result<IwLink, UnavailableReason> {
    let mut lines = output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty());
    let header = lines.next().ok_or(UnavailableReason::InvalidMeasurement)?;

    if header == "Not connected." {
        return Err(UnavailableReason::NotMeasured);
    }

    let bssid = header
        .strip_prefix("Connected to ")
        .and_then(|tail| tail.split_ascii_whitespace().next())
        .ok_or(UnavailableReason::InvalidMeasurement)?;

    let is_mac = bssid.len() == 17
        && bssid.split(':').count() == 6
        && bssid
            .split(':')
            .all(|octet| octet.len() == 2 && octet.bytes().all(|byte| byte.is_ascii_hexdigit()));
    if !is_mac {
        return Err(UnavailableReason::InvalidMeasurement);
    }

    let mut frequency_mhz = None;
    let mut rssi_dbm = None;

    for line in lines {
        if let Some(raw) = line.strip_prefix("freq:") {
            let raw = raw.trim();
            let whole_mhz = match raw.split_once('.') {
                Some((whole, fraction))
                    if !fraction.is_empty() && fraction.bytes().all(|byte| byte == b'0') =>
                {
                    whole
                }
                Some(_) => return Err(UnavailableReason::InvalidMeasurement),
                None => raw,
            };

            let value = whole_mhz
                .parse::<u32>()
                .map_err(|_| UnavailableReason::InvalidMeasurement)?;

            if value == 0 || frequency_mhz.replace(value).is_some() {
                return Err(UnavailableReason::InvalidMeasurement);
            }
        } else if let Some(raw) = line.strip_prefix("signal:") {
            let mut fields = raw.split_ascii_whitespace();
            let value = fields
                .next()
                .and_then(|field| field.parse::<i16>().ok())
                .filter(|value| *value < 0)
                .ok_or(UnavailableReason::InvalidMeasurement)?;

            if fields.next() != Some("dBm")
                || fields.next().is_some()
                || rssi_dbm.replace(value).is_some()
            {
                return Err(UnavailableReason::InvalidMeasurement);
            }
        }
    }

    Ok(IwLink {
        bssid: bssid.to_owned(),
        frequency_mhz: frequency_mhz.ok_or(UnavailableReason::NotMeasured)?,
        rssi_dbm: rssi_dbm.ok_or(UnavailableReason::NotMeasured)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINK: &str = "Connected to aa:bb:cc:dd:ee:ff (on wlan0)\n\
                       \tSSID: Example\n\tfreq: 5260\n\tsignal: -57 dBm\n";

    #[test]
    fn parses_reported_dbm() {
        let link = parse_link(LINK).unwrap();
        assert_eq!(link.bssid, "aa:bb:cc:dd:ee:ff");
        assert_eq!(link.frequency_mhz, 5260);
        assert_eq!(link.rssi_dbm, -57);
    }

    #[test]
    fn rejects_non_dbm_signal() {
        assert!(matches!(
            parse_link(&LINK.replace("-57 dBm", "62 %")),
            Err(UnavailableReason::InvalidMeasurement)
        ));
    }

    #[test]
    fn reports_disconnection() {
        assert!(matches!(
            parse_link("Not connected.\n"),
            Err(UnavailableReason::NotMeasured)
        ));
    }

    #[test]
    fn parses_frequency_with_zero_fraction() {
        let output = LINK.replace("freq: 5260", "freq: 5260.0");
        let link = parse_link(&output).expect("valid iw link");

        assert_eq!(link.frequency_mhz, 5260);
        assert_eq!(link.rssi_dbm, -57);
    }
}
