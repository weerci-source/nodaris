use nodaris_core::models::measurement::UnavailableReason;

use super::command;

pub(super) struct IwLink {
    pub(super) bssid: String,
    pub(super) frequency_mhz: u32,
    pub(super) rssi_dbm: Option<i16>,
}

impl IwLink {
    pub(super) fn same_context(&self, other: &Self) -> bool {
        self.bssid.eq_ignore_ascii_case(&other.bssid) && self.frequency_mhz == other.frequency_mhz
    }
}

pub(super) enum LinkState {
    Disconnected,
    Connected { link: IwLink, ssid: Option<String> },
}

pub(super) fn read_link(interface: &str) -> Result<IwLink, UnavailableReason> {
    match read_link_state(interface)? {
        LinkState::Connected { link, .. } => Ok(link),
        LinkState::Disconnected => Err(UnavailableReason::NotMeasured),
    }
}

pub(super) fn read_link_state(interface: &str) -> Result<LinkState, UnavailableReason> {
    parse_link_state(&command::read_iw(&["dev", interface, "link"])?)
}

pub(super) fn parse_link_state(output: &str) -> Result<LinkState, UnavailableReason> {
    let mut lines = output
        .lines()
        .map(str::trim_start)
        .filter(|line| !line.is_empty());
    let header = lines.next().ok_or(UnavailableReason::InvalidMeasurement)?;

    if header.trim_end() == "Not connected." {
        return Ok(LinkState::Disconnected);
    }

    let bssid = header
        .strip_prefix("Connected to ")
        .and_then(|tail| tail.split_ascii_whitespace().next())
        .ok_or(UnavailableReason::InvalidMeasurement)?;

    if !valid_bssid(bssid) {
        return Err(UnavailableReason::InvalidMeasurement);
    }

    let mut ssid = None;
    let mut frequency_mhz = None;
    let mut rssi_dbm = None;

    for line in lines {
        if let Some(value) = line.strip_prefix("SSID: ") {
            if ssid.replace(value.to_string()).is_some() {
                return Err(UnavailableReason::InvalidMeasurement);
            }
        } else if let Some(raw) = line.strip_prefix("freq:") {
            let value = parse_frequency(raw).ok_or(UnavailableReason::InvalidMeasurement)?;
            if frequency_mhz.replace(value).is_some() {
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

    Ok(LinkState::Connected {
        link: IwLink {
            bssid: bssid.to_ascii_lowercase(),
            frequency_mhz: frequency_mhz.ok_or(UnavailableReason::NotMeasured)?,
            rssi_dbm,
        },
        ssid,
    })
}

pub(super) fn valid_bssid(value: &str) -> bool {
    value.len() == 17
        && value.split(':').count() == 6
        && value
            .split(':')
            .all(|octet| octet.len() == 2 && octet.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

pub(super) fn parse_frequency(value: &str) -> Option<u32> {
    let value = value.trim();
    let whole = match value.split_once('.') {
        Some((whole, fraction))
            if !fraction.is_empty() && fraction.bytes().all(|byte| byte == b'0') =>
        {
            whole
        }
        Some(_) => return None,
        None => value,
    };
    whole.parse().ok().filter(|&value| value > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINK: &str = "Connected to aa:bb:cc:dd:ee:ff (on wlan0)\n\
                       \tSSID: Example\n\tfreq: 5260\n\tsignal: -57 dBm\n";

    #[test]
    fn parses_reported_dbm() {
        let LinkState::Connected { link, ssid } = parse_link_state(LINK).unwrap() else {
            panic!("expected connected")
        };
        assert_eq!(ssid.as_deref(), Some("Example"));
        assert_eq!(link.bssid, "aa:bb:cc:dd:ee:ff");
        assert_eq!(link.frequency_mhz, 5260);
        assert_eq!(link.rssi_dbm, Some(-57));
    }

    #[test]
    fn rejects_non_dbm_signal() {
        assert!(matches!(
            parse_link_state(&LINK.replace("-57 dBm", "62 %")),
            Err(UnavailableReason::InvalidMeasurement)
        ));
    }

    #[test]
    fn reports_disconnection() {
        assert!(matches!(
            parse_link_state("Not connected.\n"),
            Ok(LinkState::Disconnected)
        ));
    }

    #[test]
    fn parses_frequency_with_zero_fraction() {
        let output = LINK.replace("freq: 5260", "freq: 5260.0");
        let LinkState::Connected { link, .. } = parse_link_state(&output).expect("valid iw link")
        else {
            panic!("expected connected")
        };

        assert_eq!(link.frequency_mhz, 5260);
        assert_eq!(link.rssi_dbm, Some(-57));
    }
}
