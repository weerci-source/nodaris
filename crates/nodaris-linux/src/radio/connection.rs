use nodaris_core::models::{
    connection::{ConnectedAccessPoint, ConnectionObservation, ConnectionState, Ssid},
    measurement::{Measurement, ObservationTime, SourceId, UnavailableReason},
    radio::RssiDbm,
    radio_observation::{Bssid, FrequencyMhz, InterfaceId},
};

use super::{
    iw_link::{self, LinkState},
    neighbors,
};

/// Reads one link snapshot without triggering a scan or reconnecting.
pub fn observe_connection(
    interface: InterfaceId,
    fresh_since_ns: Option<u64>,
    read_time: impl Fn() -> ObservationTime,
) -> ConnectionObservation {
    let link = iw_link::read_link_state(&interface.0);
    let mut result = observation(interface, link, read_time());
    if let Some(ConnectionState::Connected(ap)) = result.state.value() {
        result.neighbors = neighbors::collect(
            &result.interface.0,
            ap,
            &result.state.source,
            &result.state.observed_at,
            fresh_since_ns,
            read_time,
        );
    }
    result
}

fn observation(
    interface: InterfaceId,
    link: Result<LinkState, UnavailableReason>,
    observed_at: ObservationTime,
) -> ConnectionObservation {
    let source = SourceId("linux.iw".into());
    let state = match link {
        Ok(LinkState::Connected {
            link,
            ssid: Some(ssid),
        }) => Ok(ConnectionState::Connected(ConnectedAccessPoint {
            ssid: Ssid(ssid),
            bssid: Bssid(link.bssid),
            frequency: Some(FrequencyMhz(link.frequency_mhz)),
            rssi: link.rssi_dbm.map(|value| {
                Measurement::available(RssiDbm::new(value), source.clone(), observed_at.clone())
            }),
        })),
        Ok(LinkState::Disconnected) => Ok(ConnectionState::Disconnected),
        Ok(LinkState::Connected { ssid: None, .. }) => Err(UnavailableReason::NotMeasured),
        Err(reason) => Err(reason),
    };

    ConnectionObservation {
        interface,
        state: match state {
            Ok(state) => Measurement::available(state, source.clone(), observed_at.clone()),
            Err(reason) => Measurement::unavailable(reason, source.clone(), observed_at.clone()),
        },
        // A cached scan without verified freshness is not a neighbor measurement.
        neighbors: Measurement::unavailable(UnavailableReason::NotMeasured, source, observed_at),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nodaris_core::models::{
        connection_timeline::{ConnectionTransitionKind, summarize_connection},
        measurement::MeasurementSessionId,
    };

    #[test]
    fn replays_link_output_through_to_the_timeline() {
        let first = "Connected to 02:00:00:00:00:01 (on wlan0)\n\tSSID: Example\n\tfreq: 5260\n\tsignal: -75 dBm\n";
        let second = "Connected to 02:00:00:00:00:02 (on wlan0)\n\tSSID: Example\n\tfreq: 5180.0\n";
        let observations: Vec<_> = [first, second, "Not connected.\n", second]
            .into_iter()
            .enumerate()
            .map(|(index, output)| {
                observation(
                    InterfaceId("wlan0".into()),
                    iw_link::parse_link_state(output),
                    ObservationTime {
                        unix_time_ms: None,
                        monotonic_time_ns: 1 + index as u64 * 2_000_000_000,
                        session_id: MeasurementSessionId("replay".into()),
                    },
                )
            })
            .collect();
        let timeline = summarize_connection(&observations);
        assert_eq!(timeline.sample_count, 4);
        assert_eq!(timeline.unavailable_count, 0);
        assert_eq!(timeline.transitions.len(), 3);
        let change = &timeline.transitions[0];
        assert_eq!(change.kind, ConnectionTransitionKind::AccessPointChanged);
        assert_eq!(change.before.rssi_dbm, Some(-75));
        assert_eq!(change.after.frequency_mhz, Some(FrequencyMhz(5180)));
        assert_eq!(change.after.rssi_dbm, None);
        assert_eq!(
            timeline.transitions[1].kind,
            ConnectionTransitionKind::Disconnected
        );
        assert_eq!(
            timeline.transitions[2].kind,
            ConnectionTransitionKind::Connected
        );
    }
}
