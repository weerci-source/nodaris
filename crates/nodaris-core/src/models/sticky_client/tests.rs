use super::*;

use super::super::connection::{
    ConnectedAccessPoint, ConnectionObservation, ConnectionState, NeighborAccessPoint, Ssid,
};
use super::super::measurement::ObservationTime;
use super::super::measurement::UnavailableReason;
use super::super::measurement::{Measurement, MeasurementSessionId, MeasurementState, SourceId};
use super::super::radio::RssiDbm;
use super::super::radio_observation::{Bssid, FrequencyMhz, InterfaceId};

const SOURCE: &str = "linux.test";
const SESSION: &str = "session-1";
const SSID: &str = "Home";
const CONNECTED_BSSID: &str = "AA:BB:CC:00:00:01";
const CANDIDATE_BSSID: &str = "AA:BB:CC:00:00:02";

fn config() -> StickyClientConfig {
    StickyClientConfig::default()
}

fn observation_time(at_ns: u64) -> ObservationTime {
    ObservationTime {
        unix_time_ms: None,
        monotonic_time_ns: at_ns,
        session_id: MeasurementSessionId(SESSION.to_string()),
    }
}

fn rssi_measurement(value: i16, source: &str, at_ns: u64) -> Measurement<RssiDbm> {
    Measurement::available(
        RssiDbm::new(value),
        SourceId(source.to_string()),
        observation_time(at_ns),
    )
}

fn neighbor(bssid: &str, rssi: Option<Measurement<RssiDbm>>) -> NeighborAccessPoint {
    NeighborAccessPoint {
        ssid: Ssid(SSID.to_string()),
        bssid: Bssid(bssid.to_string()),
        frequency: FrequencyMhz(5180),
        rssi,
    }
}

fn observation(
    at_ns: u64,
    current_rssi_dbm: i16,
    neighbors: Vec<NeighborAccessPoint>,
) -> ConnectionObservation {
    let source = SourceId(SOURCE.to_string());
    let observed_at = observation_time(at_ns);

    let current_rssi = rssi_measurement(current_rssi_dbm, SOURCE, at_ns);

    ConnectionObservation {
        interface: InterfaceId("wlan0".to_string()),
        state: Measurement::available(
            ConnectionState::Connected(ConnectedAccessPoint {
                ssid: Ssid(SSID.to_string()),
                bssid: Bssid(CONNECTED_BSSID.to_string()),
                frequency: Some(FrequencyMhz(5180)),
                rssi: Some(current_rssi),
            }),
            source.clone(),
            observed_at.clone(),
        ),
        neighbors: Measurement::available(neighbors, source, observed_at),
    }
}

fn candidate(at_ns: u64, rssi_dbm: i16) -> NeighborAccessPoint {
    neighbor(
        CANDIDATE_BSSID,
        Some(rssi_measurement(rssi_dbm, SOURCE, at_ns)),
    )
}

fn stable_observations() -> Vec<ConnectionObservation> {
    vec![
        observation(0, -70, vec![candidate(0, -60)]),
        observation(15_000_000_000, -70, vec![candidate(15_000_000_000, -60)]),
        observation(30_000_000_000, -70, vec![candidate(30_000_000_000, -60)]),
    ]
}

fn set_current_rssi(
    observation: &mut ConnectionObservation,
    measurement: Option<Measurement<RssiDbm>>,
) {
    if let MeasurementState::Available(ConnectionState::Connected(ap)) =
        &mut observation.state.state
    {
        ap.rssi = measurement;
    }
}

#[test]
fn detects_stable_sticky_client_with_exact_threshold() {
    let observations = stable_observations();

    let result = assess_sticky_client(&observations, &config());

    match result {
        StickyClientAssessment::Suspected(evidence) => {
            assert_eq!(evidence.connected_bssid, Bssid(CONNECTED_BSSID.to_string()));
            assert_eq!(evidence.candidate_bssid, Bssid(CANDIDATE_BSSID.to_string()));
            assert_eq!(evidence.signal_advantage_db, 10);
            assert_eq!(evidence.minimum_signal_advantage_db, 10);
            assert_eq!(evidence.consecutive_observations, 3);
            assert_eq!(evidence.duration_ns, 30_000_000_000);
        }
        other => panic!("Expected Suspected, got {other:?}"),
    }
}

#[test]
fn returns_no_evidence_when_advantage_is_below_threshold() {
    let observations = vec![
        observation(0, -70, vec![candidate(0, -61)]),
        observation(15_000_000_000, -70, vec![candidate(15_000_000_000, -61)]),
        observation(30_000_000_000, -70, vec![candidate(30_000_000_000, -61)]),
    ];

    assert_eq!(
        assess_sticky_client(&observations, &config()),
        StickyClientAssessment::NoEvidence,
    );
}

#[test]
fn returns_no_evidence_when_advantage_is_not_stable() {
    let observations = vec![
        observation(0, -70, vec![candidate(0, -60)]),
        observation(15_000_000_000, -70, vec![candidate(15_000_000_000, -60)]),
        // Преимущество в последнем наблюдении составляет только 9 dB.
        observation(30_000_000_000, -70, vec![candidate(30_000_000_000, -61)]),
    ];

    assert_eq!(
        assess_sticky_client(&observations, &config()),
        StickyClientAssessment::NoEvidence,
    );
}

#[test]
fn rejects_candidate_missing_from_middle_observation() {
    let observations = vec![
        observation(0, -70, vec![candidate(0, -60)]),
        observation(15_000_000_000, -70, vec![]),
        observation(30_000_000_000, -70, vec![candidate(30_000_000_000, -60)]),
    ];

    assert_eq!(
        assess_sticky_client(&observations, &config()),
        StickyClientAssessment::NoEvidence,
    );
}

#[test]
fn ignores_candidate_that_appears_only_later() {
    let observations = vec![
        observation(0, -70, vec![]),
        observation(15_000_000_000, -70, vec![candidate(15_000_000_000, -60)]),
        observation(30_000_000_000, -70, vec![candidate(30_000_000_000, -60)]),
    ];

    assert_eq!(
        assess_sticky_client(&observations, &config()),
        StickyClientAssessment::NoEvidence,
    );
}

#[test]
fn valid_candidate_is_detected_even_if_another_candidate_has_no_rssi() {
    let observations = [0, 15_000_000_000, 30_000_000_000]
        .into_iter()
        .map(|at_ns| {
            observation(
                at_ns,
                -70,
                vec![candidate(at_ns, -60), neighbor("AA:BB:CC:00:00:03", None)],
            )
        })
        .collect::<Vec<_>>();

    assert!(matches!(
        assess_sticky_client(&observations, &config()),
        StickyClientAssessment::Suspected(_),
    ));
}

#[test]
fn reports_unavailable_neighbor_rssi_when_no_candidate_can_be_checked() {
    let observations = [0, 15_000_000_000, 30_000_000_000]
        .into_iter()
        .map(|at_ns| observation(at_ns, -70, vec![neighbor(CANDIDATE_BSSID, None)]))
        .collect::<Vec<_>>();

    assert_eq!(
        assess_sticky_client(&observations, &config()),
        StickyClientAssessment::InsufficientData(
            StickyClientUnavailableReason::NeighborRssiUnavailable,
        ),
    );
}

#[test]
fn reports_unavailable_current_rssi() {
    let mut observations = stable_observations();

    let unavailable = Measurement::unavailable(
        UnavailableReason::TemporarilyUnavailable,
        SourceId(SOURCE.to_string()),
        observation_time(15_000_000_000),
    );

    set_current_rssi(&mut observations[1], Some(unavailable));

    assert_eq!(
        assess_sticky_client(&observations, &config()),
        StickyClientAssessment::InsufficientData(
            StickyClientUnavailableReason::MeasurementUnavailable(
                UnavailableReason::TemporarilyUnavailable,
            ),
        ),
    );
}

#[test]
fn reports_missing_current_rssi() {
    let mut observations = stable_observations();

    set_current_rssi(&mut observations[1], None);

    assert_eq!(
        assess_sticky_client(&observations, &config()),
        StickyClientAssessment::InsufficientData(
            StickyClientUnavailableReason::CurrentRssiUnavailable,
        ),
    );
}

#[test]
fn reports_unavailable_neighbor_scan() {
    let mut observations = stable_observations();

    observations[1].neighbors.state =
        MeasurementState::Unavailable(UnavailableReason::TemporarilyUnavailable);

    assert_eq!(
        assess_sticky_client(&observations, &config()),
        StickyClientAssessment::InsufficientData(
            StickyClientUnavailableReason::MeasurementUnavailable(
                UnavailableReason::TemporarilyUnavailable,
            ),
        ),
    );
}

#[test]
fn rejects_stale_current_rssi() {
    let mut observations = stable_observations();

    set_current_rssi(&mut observations[1], Some(rssi_measurement(-70, SOURCE, 0)));

    assert_eq!(
        assess_sticky_client(&observations, &config()),
        StickyClientAssessment::InsufficientData(StickyClientUnavailableReason::StaleMeasurement,),
    );
}

#[test]
fn rejects_current_rssi_from_too_far_in_the_future() {
    let mut observations = stable_observations();

    set_current_rssi(
        &mut observations[1],
        Some(rssi_measurement(-70, SOURCE, 30_000_000_000)),
    );

    assert_eq!(
        assess_sticky_client(&observations, &config()),
        StickyClientAssessment::InsufficientData(StickyClientUnavailableReason::FutureMeasurement,),
    );
}

#[test]
fn rejects_measurement_from_different_source() {
    let mut observations = stable_observations();

    set_current_rssi(
        &mut observations[1],
        Some(rssi_measurement(-70, "another.provider", 15_000_000_000)),
    );

    assert_eq!(
        assess_sticky_client(&observations, &config()),
        StickyClientAssessment::InsufficientData(StickyClientUnavailableReason::DifferentSources,),
    );
}

#[test]
fn rejects_different_observation_sources() {
    let mut observations = stable_observations();
    observations[1].state.source = SourceId("another.provider".to_string());

    assert_eq!(
        assess_sticky_client(&observations, &config()),
        StickyClientAssessment::InsufficientData(StickyClientUnavailableReason::DifferentSources,),
    );
}

#[test]
fn rejects_different_sessions() {
    let mut observations = stable_observations();

    observations[1].state.observed_at.session_id =
        MeasurementSessionId("another-session".to_string());

    assert_eq!(
        assess_sticky_client(&observations, &config()),
        StickyClientAssessment::InsufficientData(StickyClientUnavailableReason::DifferentSessions,),
    );
}

#[test]
fn rejects_non_increasing_observation_time() {
    let mut observations = stable_observations();

    observations[1].state.observed_at.monotonic_time_ns = 0;

    assert_eq!(
        assess_sticky_client(&observations, &config()),
        StickyClientAssessment::InsufficientData(StickyClientUnavailableReason::NonIncreasingTime,),
    );
}

#[test]
fn rejects_connection_change() {
    let mut observations = stable_observations();

    if let MeasurementState::Available(ConnectionState::Connected(ap)) =
        &mut observations[1].state.state
    {
        ap.bssid = Bssid("AA:BB:CC:00:00:99".to_string());
    }

    assert_eq!(
        assess_sticky_client(&observations, &config()),
        StickyClientAssessment::InsufficientData(StickyClientUnavailableReason::ConnectionChanged,),
    );
}

#[test]
fn rejects_network_change() {
    let mut observations = stable_observations();

    if let MeasurementState::Available(ConnectionState::Connected(ap)) =
        &mut observations[1].state.state
    {
        ap.ssid = Ssid("Guest".to_string());
    }

    assert_eq!(
        assess_sticky_client(&observations, &config()),
        StickyClientAssessment::InsufficientData(StickyClientUnavailableReason::DifferentNetworks,),
    );
}

#[test]
fn rejects_disconnected_observation() {
    let mut observations = stable_observations();

    observations[1].state.state = MeasurementState::Available(ConnectionState::Disconnected);

    assert_eq!(
        assess_sticky_client(&observations, &config()),
        StickyClientAssessment::InsufficientData(StickyClientUnavailableReason::NotConnected,),
    );
}

#[test]
fn rejects_insufficient_observation_count() {
    let observations = stable_observations();

    assert_eq!(
        assess_sticky_client(&observations[..2], &config()),
        StickyClientAssessment::InsufficientData(
            StickyClientUnavailableReason::InsufficientObservations,
        ),
    );
}

#[test]
fn rejects_insufficient_duration() {
    let observations = vec![
        observation(0, -70, vec![candidate(0, -60)]),
        observation(10_000_000_000, -70, vec![candidate(10_000_000_000, -60)]),
        observation(20_000_000_000, -70, vec![candidate(20_000_000_000, -60)]),
    ];

    assert_eq!(
        assess_sticky_client(&observations, &config()),
        StickyClientAssessment::InsufficientData(
            StickyClientUnavailableReason::InsufficientDuration,
        ),
    );
}

#[test]
fn rejects_invalid_configuration() {
    let observations = stable_observations();
    let mut invalid_config = config();
    invalid_config.min_signal_advantage_db = 0;

    assert_eq!(
        assess_sticky_client(&observations, &invalid_config),
        StickyClientAssessment::InsufficientData(
            StickyClientUnavailableReason::InvalidConfiguration,
        ),
    );
}

#[test]
fn accepts_current_rssi_within_future_tolerance() {
    let mut observations = stable_observations();

    set_current_rssi(
        &mut observations[1],
        Some(rssi_measurement(-70, SOURCE, 25_000_000_000)),
    );

    assert!(matches!(
        assess_sticky_client(&observations, &config()),
        StickyClientAssessment::Suspected(_),
    ));
}

#[test]
fn accepts_neighbor_rssi_within_past_tolerance() {
    let mut observations = stable_observations();

    observations[1].neighbors.state =
        MeasurementState::Available(vec![candidate(5_000_000_000, -60)]);

    assert!(matches!(
        assess_sticky_client(&observations, &config()),
        StickyClientAssessment::Suspected(_),
    ));
}

#[test]
fn rejects_excessive_observation_gap() {
    let observations = vec![
        observation(0, -70, vec![candidate(0, -60)]),
        observation(15_000_000_000, -70, vec![candidate(15_000_000_000, -60)]),
        observation(60_000_000_000, -70, vec![candidate(60_000_000_000, -60)]),
    ];

    assert_eq!(
        assess_sticky_client(&observations, &config()),
        StickyClientAssessment::InsufficientData(
            StickyClientUnavailableReason::ObservationGapExceeded,
        ),
    );
}

#[test]
fn accepts_observation_gap_at_exact_limit() {
    let observations = vec![
        observation(0, -70, vec![candidate(0, -60)]),
        observation(20_000_000_000, -70, vec![candidate(20_000_000_000, -60)]),
        observation(40_000_000_000, -70, vec![candidate(40_000_000_000, -60)]),
    ];

    assert!(matches!(
        assess_sticky_client(&observations, &config()),
        StickyClientAssessment::Suspected(_),
    ));
}

#[test]
fn report_no_evidence_has_no_neighbor_issues() {
    let observations = [0, 15_000_000_000, 30_000_000_000]
        .into_iter()
        .map(|at_ns| observation(at_ns, -70, vec![candidate(at_ns, -61)]))
        .collect::<Vec<_>>();

    let report = assess_sticky_client_report(&observations, &config());

    assert_eq!(report.assessment, StickyClientAssessment::NoEvidence);
    assert_eq!(report.observation_count, 3);
    assert!(report.neighbor_issues.is_empty());
}

#[test]
fn report_suspected_preserves_other_neighbor_issues() {
    const UNAVAILABLE_BSSID: &str = "AA:BB:CC:00:00:03";

    let times = [0, 15_000_000_000, 30_000_000_000];

    let observations = times
        .into_iter()
        .map(|at_ns| {
            observation(
                at_ns,
                -70,
                vec![candidate(at_ns, -60), neighbor(UNAVAILABLE_BSSID, None)],
            )
        })
        .collect::<Vec<_>>();

    let report = assess_sticky_client_report(&observations, &config());

    assert!(matches!(
        report.assessment,
        StickyClientAssessment::Suspected(_)
    ));

    assert_eq!(report.observation_count, 3);
    assert_eq!(report.neighbor_issues.len(), 3);

    for (index, observed_issue) in report.neighbor_issues.iter().enumerate() {
        assert_eq!(observed_issue.observation_index, index);
        assert_eq!(observed_issue.monotonic_time_ns, times[index]);

        assert_eq!(
            observed_issue.issue.bssid,
            Bssid(UNAVAILABLE_BSSID.to_string())
        );

        assert_eq!(
            observed_issue.issue.reason,
            StickyClientUnavailableReason::NeighborRssiUnavailable
        );
    }
}

#[test]
fn report_insufficient_data_preserves_neighbor_issues() {
    let times = [0, 15_000_000_000, 30_000_000_000];

    let observations = times
        .into_iter()
        .map(|at_ns| observation(at_ns, -70, vec![neighbor(CANDIDATE_BSSID, None)]))
        .collect::<Vec<_>>();

    let report = assess_sticky_client_report(&observations, &config());

    assert_eq!(
        report.assessment,
        StickyClientAssessment::InsufficientData(
            StickyClientUnavailableReason::NeighborRssiUnavailable
        )
    );

    assert_eq!(report.observation_count, 3);
    assert_eq!(report.neighbor_issues.len(), 3);

    for (index, observed_issue) in report.neighbor_issues.iter().enumerate() {
        assert_eq!(observed_issue.observation_index, index);
        assert_eq!(observed_issue.monotonic_time_ns, times[index]);

        assert_eq!(
            observed_issue.issue.bssid,
            Bssid(CANDIDATE_BSSID.to_string())
        );

        assert_eq!(
            observed_issue.issue.reason,
            StickyClientUnavailableReason::NeighborRssiUnavailable
        );
    }
}

#[test]
fn report_early_termination_preserves_accepted_neighbor_issues() {
    const UNAVAILABLE_BSSID: &str = "AA:BB:CC:00:00:03";

    let mut observations = stable_observations();

    observations[0].neighbors.state =
        MeasurementState::Available(vec![candidate(0, -60), neighbor(UNAVAILABLE_BSSID, None)]);

    // Второе наблюдение имеет то же время, что и первое.
    // Анализ должен завершиться до извлечения второго образца.
    observations[1].state.observed_at.monotonic_time_ns = 0;

    let report = assess_sticky_client_report(&observations, &config());

    assert_eq!(
        report.assessment,
        StickyClientAssessment::InsufficientData(StickyClientUnavailableReason::NonIncreasingTime)
    );

    // Передано три наблюдения, но принят только первый образец.
    assert_eq!(report.observation_count, 3);
    assert_eq!(report.neighbor_issues.len(), 1);

    let observed_issue = &report.neighbor_issues[0];

    assert_eq!(observed_issue.observation_index, 0);
    assert_eq!(observed_issue.monotonic_time_ns, 0);

    assert_eq!(
        observed_issue.issue.bssid,
        Bssid(UNAVAILABLE_BSSID.to_string())
    );

    assert_eq!(
        observed_issue.issue.reason,
        StickyClientUnavailableReason::NeighborRssiUnavailable
    );
}

#[test]
fn report_invalid_configuration_does_not_process_observations() {
    let observations = stable_observations();

    let mut invalid_config = config();
    invalid_config.min_signal_advantage_db = 0;

    let report = assess_sticky_client_report(&observations, &invalid_config);

    assert_eq!(
        report.assessment,
        StickyClientAssessment::InsufficientData(
            StickyClientUnavailableReason::InvalidConfiguration
        )
    );

    assert_eq!(report.observation_count, observations.len());
    assert!(report.neighbor_issues.is_empty());
}

#[test]
fn report_preserves_multiple_neighbor_issues_in_same_observation() {
    const FIRST_BSSID: &str = "AA:BB:CC:00:00:03";
    const SECOND_BSSID: &str = "AA:BB:CC:00:00:04";

    let observations = [0, 15_000_000_000, 30_000_000_000]
        .into_iter()
        .map(|at_ns| {
            let mut neighbors = vec![candidate(at_ns, -60)];

            if at_ns == 15_000_000_000 {
                neighbors.push(neighbor(FIRST_BSSID, None));
                neighbors.push(neighbor(SECOND_BSSID, None));
            }

            observation(at_ns, -70, neighbors)
        })
        .collect::<Vec<_>>();

    let report = assess_sticky_client_report(&observations, &config());

    assert!(matches!(
        report.assessment,
        StickyClientAssessment::Suspected(_)
    ));

    assert_eq!(report.observation_count, 3);
    assert_eq!(report.neighbor_issues.len(), 2);

    let first = &report.neighbor_issues[0];
    let second = &report.neighbor_issues[1];

    assert_eq!(first.observation_index, 1);
    assert_eq!(second.observation_index, 1);

    assert_eq!(first.monotonic_time_ns, 15_000_000_000);
    assert_eq!(second.monotonic_time_ns, 15_000_000_000);

    assert_eq!(first.issue.bssid, Bssid(FIRST_BSSID.to_string()));
    assert_eq!(second.issue.bssid, Bssid(SECOND_BSSID.to_string()));

    assert_eq!(
        first.issue.reason,
        StickyClientUnavailableReason::NeighborRssiUnavailable
    );

    assert_eq!(
        second.issue.reason,
        StickyClientUnavailableReason::NeighborRssiUnavailable
    );
}
