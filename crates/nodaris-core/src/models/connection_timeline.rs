use serde::Serialize;

use super::{
    connection::{
        ConnectionObservation, ConnectionState, RoamingDetection, RoamingDetectionMethod,
        RoamingUnavailableReason, detect_roaming,
    },
    measurement::{ObservationTime, SourceId},
    radio_observation::{Bssid, FrequencyMhz},
    sticky_client::{StickyClientConfig, StickyClientReport, assess_sticky_client_report},
};

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionTransitionKind {
    AccessPointChanged,
    NetworkChanged,
    Connected,
    Disconnected,
}

#[derive(Debug, Serialize)]
pub struct ConnectionEndpoint {
    pub observed_at: ObservationTime,
    pub bssid: Option<Bssid>,
    pub frequency_mhz: Option<FrequencyMhz>,
    pub rssi_dbm: Option<i16>,
}

#[derive(Debug, Serialize)]
pub struct ConnectionTransition {
    pub kind: ConnectionTransitionKind,
    pub detection_method: RoamingDetectionMethod,
    pub source: SourceId,
    pub before: ConnectionEndpoint,
    pub after: ConnectionEndpoint,
}

#[derive(Debug, Serialize)]
pub struct ConnectionTimeline {
    pub sample_count: usize,
    pub unavailable_count: usize,
    pub transitions: Vec<ConnectionTransition>,
    pub sticky_client: StickyClientReport,
}

#[derive(Serialize)]
pub struct ConnectionRun {
    pub interface: String,
    pub elapsed_ms: u64,
    #[serde(flatten)]
    pub timeline: ConnectionTimeline,
}

pub fn summarize_connection(observations: &[ConnectionObservation]) -> ConnectionTimeline {
    let config = StickyClientConfig::default();
    let transitions = observations
        .windows(2)
        .filter_map(|pair| transition(&pair[0], &pair[1], config.max_observation_gap_ns))
        .collect();
    let stable_pairs = observations
        .windows(2)
        .rev()
        .take_while(|pair| same_connection(&pair[0], &pair[1], config.max_observation_gap_ns))
        .count();
    let mut window_start = observations.len().saturating_sub(stable_pairs + 1);
    // A gap in fresh evidence starts a new window. If the last read itself
    // failed, retain it so the assessment still reports insufficient data.
    if observations
        .last()
        .is_some_and(|last| last.neighbors.value().is_some())
        && let Some(gap) = observations[window_start..]
            .iter()
            .rposition(|item| item.neighbors.value().is_none())
    {
        window_start += gap + 1;
    }
    ConnectionTimeline {
        sample_count: observations.len(),
        unavailable_count: observations
            .iter()
            .filter(|item| item.state.value().is_none())
            .count(),
        transitions,
        sticky_client: assess_sticky_client_report(&observations[window_start..], &config),
    }
}

fn transition(
    before: &ConnectionObservation,
    after: &ConnectionObservation,
    max_gap_ns: u64,
) -> Option<ConnectionTransition> {
    if !within_gap(before, after, max_gap_ns) {
        return None;
    }
    let kind = match detect_roaming(before, after) {
        Ok(RoamingDetection::Roamed(_)) => ConnectionTransitionKind::AccessPointChanged,
        Ok(RoamingDetection::ConnectionEstablished) => ConnectionTransitionKind::Connected,
        Ok(RoamingDetection::ConnectionLost) => ConnectionTransitionKind::Disconnected,
        Err(RoamingUnavailableReason::DifferentNetworks) => {
            ConnectionTransitionKind::NetworkChanged
        }
        _ => return None,
    };
    Some(ConnectionTransition {
        kind,
        detection_method: RoamingDetectionMethod::SnapshotComparison,
        source: after.state.source.clone(),
        before: endpoint(before),
        after: endpoint(after),
    })
}

fn same_connection(
    before: &ConnectionObservation,
    after: &ConnectionObservation,
    max_gap_ns: u64,
) -> bool {
    let (Some(ConnectionState::Connected(previous)), Some(ConnectionState::Connected(current))) =
        (before.state.value(), after.state.value())
    else {
        return false;
    };
    within_gap(before, after, max_gap_ns)
        && matches!(
            detect_roaming(before, after),
            Ok(RoamingDetection::NoChange)
        )
        && previous.frequency.is_some()
        && previous.frequency == current.frequency
        && previous
            .rssi
            .as_ref()
            .and_then(|value| value.value())
            .is_some()
        && current
            .rssi
            .as_ref()
            .and_then(|value| value.value())
            .is_some()
}

fn within_gap(
    before: &ConnectionObservation,
    after: &ConnectionObservation,
    max_gap_ns: u64,
) -> bool {
    let elapsed = after
        .state
        .observed_at
        .monotonic_time_ns
        .checked_sub(before.state.observed_at.monotonic_time_ns);
    elapsed.is_some_and(|gap| gap > 0 && gap <= max_gap_ns)
}

fn endpoint(observation: &ConnectionObservation) -> ConnectionEndpoint {
    let connected = match observation.state.value() {
        Some(ConnectionState::Connected(ap)) => Some(ap),
        _ => None,
    };
    let rssi_dbm = connected
        .and_then(|ap| ap.rssi.as_ref())
        .filter(|measurement| {
            measurement.source == observation.state.source
                && measurement.observed_at == observation.state.observed_at
        })
        .and_then(|measurement| measurement.value())
        .map(|rssi| rssi.as_dbm());
    ConnectionEndpoint {
        observed_at: observation.state.observed_at.clone(),
        bssid: connected.map(|ap| ap.bssid.clone()),
        frequency_mhz: connected.and_then(|ap| ap.frequency),
        rssi_dbm,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        connection::{ConnectedAccessPoint, Ssid},
        measurement::{Measurement, MeasurementSessionId, UnavailableReason},
        radio::RssiDbm,
        radio_observation::InterfaceId,
        sticky_client::{StickyClientAssessment, StickyClientUnavailableReason},
    };

    fn sample(second: u64, state: Option<(&str, &str, i16)>) -> ConnectionObservation {
        let source = SourceId("linux.iw".into());
        let time = ObservationTime {
            unix_time_ms: None,
            monotonic_time_ns: second * 1_000_000_000,
            session_id: MeasurementSessionId("test".into()),
        };
        let state = state.map_or(ConnectionState::Disconnected, |(ssid, bssid, rssi)| {
            ConnectionState::Connected(ConnectedAccessPoint {
                ssid: Ssid(ssid.into()),
                bssid: Bssid(bssid.into()),
                frequency: Some(FrequencyMhz(5260)),
                rssi: Some(Measurement::available(
                    RssiDbm::new(rssi),
                    source.clone(),
                    time.clone(),
                )),
            })
        });
        ConnectionObservation {
            interface: InterfaceId("wlan0".into()),
            state: Measurement::available(state, source.clone(), time.clone()),
            neighbors: Measurement::unavailable(UnavailableReason::NotMeasured, source, time),
        }
    }

    #[test]
    fn records_transitions_and_the_observation_endpoints() {
        let report = summarize_connection(&[
            sample(1, Some(("Office", "AP-A", -72))),
            sample(3, Some(("Office", "AP-B", -47))),
            sample(5, None),
            sample(7, Some(("Office", "AP-B", -50))),
            sample(9, Some(("Guest", "AP-C", -68))),
        ]);
        let transitions = report.transitions;
        assert_eq!(
            transitions[0].kind,
            ConnectionTransitionKind::AccessPointChanged
        );
        assert_eq!(transitions[0].before.rssi_dbm, Some(-72));
        assert_eq!(transitions[0].after.rssi_dbm, Some(-47));
        assert_eq!(
            transitions[0].detection_method,
            RoamingDetectionMethod::SnapshotComparison
        );
        assert_eq!(transitions[1].kind, ConnectionTransitionKind::Disconnected);
        assert_eq!(transitions[2].kind, ConnectionTransitionKind::Connected);
        assert_eq!(
            transitions[3].kind,
            ConnectionTransitionKind::NetworkChanged
        );
    }

    #[test]
    fn does_not_infer_transitions_across_unknown_or_widely_spaced_readings() {
        let mut unknown = sample(2, None);
        unknown.state = Measurement::unavailable(
            UnavailableReason::PermissionDenied,
            unknown.state.source,
            unknown.state.observed_at,
        );
        let report = summarize_connection(&[
            sample(1, Some(("Office", "AP-A", -65))),
            unknown,
            sample(3, Some(("Office", "AP-B", -55))),
            sample(40, None),
        ]);
        assert!(report.transitions.is_empty());
        assert_eq!(report.unavailable_count, 1);
    }

    #[test]
    fn forwards_missing_neighbor_observations_to_sticky_client() {
        let report = summarize_connection(&[
            sample(1, Some(("Office", "AP-A", -65))),
            sample(5, Some(("Office", "AP-B", -55))),
            sample(20, Some(("Office", "AP-B", -53))),
            sample(35, Some(("Office", "AP-B", -57))),
        ]);
        assert_eq!(report.sticky_client.observation_count, 3);
        assert_eq!(
            report.sticky_client.assessment,
            StickyClientAssessment::InsufficientData(
                StickyClientUnavailableReason::MeasurementUnavailable(
                    UnavailableReason::NotMeasured
                ),
            )
        );
    }
    #[test]
    fn assesses_the_same_fresh_candidate_over_a_stable_window() {
        use crate::models::connection::NeighborAccessPoint;

        let readings = |candidate_rssi: i16| {
            [1, 16, 31].map(|second| {
                let mut observation = sample(second, Some(("Office", "AP-A", -70)));
                let time = observation.state.observed_at.clone();
                let source = observation.state.source.clone();
                observation.neighbors = Measurement::available(
                    vec![NeighborAccessPoint {
                        bssid: Bssid("AP-B".into()),
                        ssid: Ssid("Office".into()),
                        frequency: FrequencyMhz(5180),
                        rssi: Some(Measurement::available(
                            RssiDbm::new(candidate_rssi),
                            source.clone(),
                            time.clone(),
                        )),
                    }],
                    source,
                    time,
                );
                observation
            })
        };
        assert!(matches!(
            summarize_connection(&readings(-59))
                .sticky_client
                .assessment,
            StickyClientAssessment::Suspected(_)
        ));
        assert_eq!(
            summarize_connection(&readings(-63))
                .sticky_client
                .assessment,
            StickyClientAssessment::NoEvidence,
        );
    }
}
