use std::{thread, time::Duration};

use nodaris_core::models::{
    diagnostic_report::ReportSection,
    measurement::{Measurement, ObservationTime, SourceId, UnavailableReason},
    radio_observation::InterfaceId,
    retries::{
        RetryCalculation, RetryCounterSemantics, RetryCounters, RetrySnapshot,
        RetryUnavailableReason, calculate_retry_deltas,
    },
};

use super::{
    command,
    iw_link::{self, IwLink},
};

const SAMPLE_GAP: Duration = Duration::from_secs(1);

struct StationCounters {
    retries: Option<u64>,
    failed: Option<u64>,
    associated_at_ns: Option<u64>,
}

pub(super) fn collect_retries(
    interface: &InterfaceId,
    initial_link: &IwLink,
    read_time: impl Fn() -> ObservationTime,
) -> ReportSection<RetryCalculation> {
    let calculation = collect_pair(interface, initial_link, read_time);
    match calculation {
        Ok(calculation) => ReportSection::Available(calculation),
        Err(reason) => ReportSection::Unavailable(reason),
    }
}

fn collect_pair(
    interface: &InterfaceId,
    initial_link: &IwLink,
    read_time: impl Fn() -> ObservationTime,
) -> Result<RetryCalculation, UnavailableReason> {
    let previous = read_station(interface, initial_link)?;
    let previous_time = read_time();
    if previous.associated_at_ns.is_none() {
        return Ok(RetryCalculation::Unavailable(
            RetryUnavailableReason::AssociationUnverified,
        ));
    }
    if previous.retries.is_none() && previous.failed.is_none() {
        return Ok(RetryCalculation::Unavailable(
            RetryUnavailableReason::MissingCounters,
        ));
    }
    thread::sleep(SAMPLE_GAP);
    let current = read_station(interface, initial_link)?;
    let current_time = read_time();
    let current_link = match iw_link::read_link(&interface.0) {
        Ok(link) => link,
        Err(UnavailableReason::NotMeasured) => {
            return Ok(RetryCalculation::Unavailable(
                RetryUnavailableReason::ConnectionChanged,
            ));
        }
        Err(reason) => return Err(reason),
    };

    if !same_link_context(initial_link, &current_link) {
        return Ok(RetryCalculation::Unavailable(
            RetryUnavailableReason::ConnectionChanged,
        ));
    }
    if let Err(reason) = match_associations(previous.associated_at_ns, current.associated_at_ns) {
        return Ok(RetryCalculation::Unavailable(reason));
    }

    Ok(calculate_retry_deltas(
        &snapshot(interface, previous, previous_time),
        &snapshot(interface, current, current_time),
    ))
}

fn same_link_context(previous: &IwLink, current: &IwLink) -> bool {
    previous.bssid.eq_ignore_ascii_case(&current.bssid)
        && previous.frequency_mhz == current.frequency_mhz
}

fn match_associations(
    previous: Option<u64>,
    current: Option<u64>,
) -> Result<(), RetryUnavailableReason> {
    match (previous, current) {
        (Some(previous), Some(current)) if previous == current => Ok(()),
        (Some(_), Some(_)) => Err(RetryUnavailableReason::ConnectionChanged),
        _ => Err(RetryUnavailableReason::AssociationUnverified),
    }
}

fn snapshot(
    interface: &InterfaceId,
    counters: StationCounters,
    observed_at: ObservationTime,
) -> RetrySnapshot {
    RetrySnapshot {
        interface: interface.clone(),
        counters: Measurement::available(
            RetryCounters {
                // iw's tx packets counts MSDUs/MMPDUs. The two counters
                // below count MPDU events, so it cannot be their denominator.
                tx_attempts: None,
                tx_retries: counters.retries,
                tx_failed: counters.failed,
                retry_semantics: RetryCounterSemantics::RetryAttempts,
            },
            SourceId("linux.iw.station".into()),
            observed_at,
        ),
    }
}

fn read_station(
    interface: &InterfaceId,
    link: &IwLink,
) -> Result<StationCounters, UnavailableReason> {
    let output = command::read_iw(&["dev", &interface.0, "station", "get", &link.bssid])?;
    parse_station(&output, &interface.0, &link.bssid)
}

fn parse_station(
    output: &str,
    interface: &str,
    bssid: &str,
) -> Result<StationCounters, UnavailableReason> {
    let mut lines = output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty());
    let Some(header) = lines.next() else {
        return Err(UnavailableReason::NotMeasured);
    };
    let fields: Vec<_> = header.split_ascii_whitespace().collect();
    if !matches!(fields.as_slice(), ["Station", address, "(on", device]
        if address.eq_ignore_ascii_case(bssid) && device.strip_suffix(')') == Some(interface))
    {
        return Err(UnavailableReason::InvalidMeasurement);
    }

    let mut counters = StationCounters {
        retries: None,
        failed: None,
        associated_at_ns: None,
    };
    for line in lines {
        if let Some(value) = line.strip_prefix("tx retries:") {
            read_field(&mut counters.retries, parse_u32(value))?;
        } else if let Some(value) = line.strip_prefix("tx failed:") {
            read_field(&mut counters.failed, parse_u32(value))?;
        } else if let Some(value) = line.strip_prefix("associated at [boottime]:") {
            read_field(&mut counters.associated_at_ns, parse_boottime(value))?;
        } else if line.starts_with("Station ") {
            return Err(UnavailableReason::InvalidMeasurement);
        }
    }

    Ok(counters)
}

fn read_field(field: &mut Option<u64>, value: Option<u64>) -> Result<(), UnavailableReason> {
    if field.is_some() || value.is_none() {
        return Err(UnavailableReason::InvalidMeasurement);
    }
    *field = value;
    Ok(())
}

fn parse_u32(value: &str) -> Option<u64> {
    value.trim().parse::<u32>().ok().map(u64::from)
}

fn parse_boottime(value: &str) -> Option<u64> {
    let value = value.trim().strip_suffix('s')?;
    let (seconds, fraction) = value.split_once('.').unwrap_or((value, "0"));
    if fraction.is_empty()
        || fraction.len() > 9
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let nanos = fraction.parse::<u64>().ok()? * 10_u64.pow(9 - fraction.len() as u32);
    seconds
        .parse::<u64>()
        .ok()?
        .checked_mul(1_000_000_000)?
        .checked_add(nanos)
}

#[cfg(test)]
mod tests {
    use super::*;

    const STATION: &str = include_str!("fixtures/station-wlan0.txt");
    const BSSID: &str = "02:00:00:00:00:01";

    #[test]
    fn parses_the_anonymized_station_snapshot() {
        let counters = parse_station(STATION, "wlan0", BSSID).unwrap();
        assert_eq!(counters.retries, Some(0));
        assert_eq!(counters.failed, Some(0));
        assert_eq!(counters.associated_at_ns, Some(43_829_604_000_000));
    }

    #[test]
    fn rejects_another_peer_interface_and_duplicate_counter() {
        let other_peer = parse_station(STATION, "wlan0", "02:00:00:00:00:00");
        let other_interface = parse_station(STATION, "wlan1", BSSID);
        let duplicated_counter =
            parse_station(&format!("{STATION}\ttx retries: 42"), "wlan0", BSSID);
        for result in [other_peer, other_interface, duplicated_counter] {
            assert!(matches!(result, Err(UnavailableReason::InvalidMeasurement)));
        }
    }

    #[test]
    fn keeps_a_missing_metric_missing_and_checks_reassociation() {
        let counters =
            parse_station(&STATION.replace("\ttx retries:\t0\n", ""), "wlan0", BSSID).unwrap();
        assert_eq!(counters.retries, None);
        assert_eq!(
            match_associations(counters.associated_at_ns, Some(42)),
            Err(RetryUnavailableReason::ConnectionChanged)
        );
        assert_eq!(
            match_associations(counters.associated_at_ns, None),
            Err(RetryUnavailableReason::AssociationUnverified)
        );
    }

    #[test]
    fn detects_a_channel_change_on_the_same_access_point() {
        let previous = IwLink {
            bssid: BSSID.into(),
            frequency_mhz: 5260,
            rssi_dbm: None,
        };
        let current = IwLink {
            bssid: BSSID.into(),
            frequency_mhz: 5280,
            rssi_dbm: None,
        };
        assert!(!same_link_context(&previous, &current));
    }
}
