use nodaris_core::models::{
    connection::{ConnectedAccessPoint, NeighborAccessPoint, Ssid},
    measurement::{Measurement, ObservationTime, SourceId, UnavailableReason},
    radio::RssiDbm,
    radio_observation::{Bssid, FrequencyMhz},
};

use super::{command, iw_link::valid_bssid};

// Guard for the millisecond/jiffy granularity of the BSS age presented by iw.
const AGE_GUARD_NS: u64 = 20_000_000;
const MAX_AGE_NS: u64 = 10_000_000_000;

pub(super) fn collect(
    interface: &str,
    ap: &ConnectedAccessPoint,
    source: &SourceId,
    start: &ObservationTime,
    fresh_since_ns: Option<u64>,
    read_time: impl FnOnce() -> ObservationTime,
) -> Measurement<Vec<NeighborAccessPoint>> {
    let Some(fresh_since_ns) = fresh_since_ns else {
        return Measurement::unavailable(
            UnavailableReason::NotMeasured,
            source.clone(),
            start.clone(),
        );
    };
    let output = command::read_iw(&["dev", interface, "scan", "dump"]);
    let end = read_time();
    let result =
        output.and_then(|output| from_cache(&output, ap, source, start, &end, fresh_since_ns));
    match result {
        Ok(neighbors) => Measurement::available(neighbors, source.clone(), end),
        Err(reason) => Measurement::unavailable(reason, source.clone(), end),
    }
}

fn from_cache(
    output: &str,
    ap: &ConnectedAccessPoint,
    source: &SourceId,
    start: &ObservationTime,
    end: &ObservationTime,
    fresh_since_ns: u64,
) -> Result<Vec<NeighborAccessPoint>, UnavailableReason> {
    let mut neighbors = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let output = format!("\n{output}");
    for entry in output.split("\nBSS ").skip(1) {
        let Some((header, fields)) = entry.split_once('\n') else {
            continue;
        };
        let bssid = header
            .split('(')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        if !valid_bssid(&bssid) || ap.bssid.0.eq_ignore_ascii_case(&bssid) {
            continue;
        }
        let get = |prefix| {
            fields
                .lines()
                .find_map(|line| line.trim_start().strip_prefix(prefix))
        };
        if get("SSID: ") != Some(ap.ssid.0.as_str()) {
            continue;
        }
        if !seen.insert(bssid.clone()) {
            return Err(UnavailableReason::InvalidMeasurement);
        }
        let Some(frequency) = get("freq:").and_then(super::iw_link::parse_frequency) else {
            return Err(UnavailableReason::InvalidMeasurement);
        };
        let rssi = get("last seen:")
            .and_then(|field| field.trim().strip_suffix(" ms ago"))
            .and_then(|age| age.parse::<u64>().ok())
            .and_then(|age| fresh_offset(start, end, age, fresh_since_ns))
            .zip(get("signal:").and_then(signal_dbm))
            .map(|(offset, value)| {
                let mut time = start.clone();
                let age_from_start = start.monotonic_time_ns - offset;
                time.monotonic_time_ns = offset;
                time.unix_time_ms = start
                    .unix_time_ms
                    .and_then(|wall| wall.checked_sub((age_from_start / 1_000_000) as i64));
                Measurement::available(value, source.clone(), time)
            });
        neighbors.push(NeighborAccessPoint {
            ssid: Ssid(ap.ssid.0.clone()),
            bssid: Bssid(bssid),
            frequency: FrequencyMhz(frequency),
            rssi,
        });
    }

    // A passive cache read cannot certify that a scan found no alternatives.
    if neighbors.iter().all(|neighbor| neighbor.rssi.is_none()) {
        Err(UnavailableReason::NotMeasured)
    } else {
        Ok(neighbors)
    }
}

fn fresh_offset(
    start: &ObservationTime,
    end: &ObservationTime,
    age_ms: u64,
    fresh_since_ns: u64,
) -> Option<u64> {
    if start.session_id != end.session_id {
        return None;
    }
    let age_ns = age_ms.checked_mul(1_000_000)?.checked_add(AGE_GUARD_NS)?;
    let earliest = start.monotonic_time_ns.checked_sub(age_ns)?;
    let oldest_possible = end.monotonic_time_ns.checked_sub(earliest)?;
    (earliest > fresh_since_ns && oldest_possible <= MAX_AGE_NS).then_some(earliest)
}

fn signal_dbm(value: &str) -> Option<RssiDbm> {
    let signal = value
        .trim()
        .strip_suffix(" dBm")?
        .parse::<f64>()
        .ok()?
        .round();
    (-128.0..=-1.0)
        .contains(&signal)
        .then_some(RssiDbm::new(signal as i16))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nodaris_core::models::measurement::MeasurementSessionId;

    fn time(ms: u64) -> ObservationTime {
        ObservationTime {
            unix_time_ms: None,
            monotonic_time_ns: ms * 1_000_000,
            session_id: MeasurementSessionId("test".into()),
        }
    }

    #[test]
    fn accepts_only_an_observation_proven_new_since_last_read() {
        let start = time(2_000);
        let end = time(2_010);
        assert_eq!(
            fresh_offset(&start, &end, 150, 1_500_000_000),
            Some(1_830_000_000)
        );
        assert_eq!(fresh_offset(&start, &end, 500, 1_500_000_000), None);
        assert_eq!(fresh_offset(&start, &end, 5_000, 0), None);
        assert_eq!(signal_dbm("-59.25 dBm"), Some(RssiDbm::new(-59)));
        assert_eq!(signal_dbm("78/100"), None);
    }
    #[test]
    fn parses_iw_candidates_and_keeps_a_stale_candidate_unmeasured() {
        let output = concat!(
            "BSS 02:00:00:00:00:02(on wlan0)\n",
            "\tfreq: 5260.0\n\tsignal: -59.25 dBm\n\tlast seen: 150 ms ago\n\tSSID: Office\n",
            "BSS 02:00:00:00:00:03(on wlan0)\n",
            "\tfreq: 5180\n\tsignal: -62.00 dBm\n\tlast seen: 520 ms ago\n\tSSID: Office\n",
        );
        let ap = ConnectedAccessPoint {
            ssid: Ssid("Office".into()),
            bssid: Bssid("02:00:00:00:00:01".into()),
            frequency: Some(FrequencyMhz(5260)),
            rssi: None,
        };
        let result = from_cache(
            output,
            &ap,
            &SourceId("linux.iw".into()),
            &time(2_000),
            &time(2_010),
            1_500_000_000,
        )
        .unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(
            result[0].rssi.as_ref().and_then(Measurement::value),
            Some(&RssiDbm::new(-59))
        );
        assert!(result[1].rssi.is_none());
    }
}
