use serde::{Deserialize, Serialize};

use super::connection::{ConnectionObservation, ConnectionState, Ssid};
use super::measurement::{
    Measurement, MeasurementSessionId, MeasurementState, SourceId, UnavailableReason,
};
use super::radio_observation::{Bssid, InterfaceId};

/// Параметры обнаружения Sticky Client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StickyClientConfig {
    /// Минимальное преимущество соседней AP в dB.
    pub min_signal_advantage_db: i32,

    /// Минимальное количество последовательных наблюдений.
    pub min_consecutive_observations: usize,

    /// Минимальная продолжительность преимущества в наносекундах.
    pub min_duration_ns: u64,

    /// Максимальный возраст измерения RSSI.
    pub max_measurement_age_ns: u64,

    /// Максимальный интервал между последовательными наблюдениями.
    pub max_observation_gap_ns: u64,
}

impl Default for StickyClientConfig {
    fn default() -> Self {
        Self {
            min_signal_advantage_db: 10,
            min_consecutive_observations: 3,
            min_duration_ns: 30_000_000_000,
            max_measurement_age_ns: 10_000_000_000,
            max_observation_gap_ns: 20_000_000_000,
        }
    }
}

/// Причина недостаточности данных.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StickyClientUnavailableReason {
    InvalidConfiguration,
    InsufficientObservations,
    InsufficientDuration,
    DifferentInterfaces,
    DifferentSources,
    DifferentSessions,
    NonIncreasingTime,
    ObservationGapExceeded,
    ConnectionUnavailable,
    NotConnected,
    ConnectionChanged,
    DifferentNetworks,
    CurrentRssiUnavailable,
    NeighborScanUnavailable,
    MeasurementUnavailable(UnavailableReason),
    StaleMeasurement,
    FutureMeasurement,
    NeighborRssiUnavailable,
}

/// Доказательства потенциального Sticky Client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StickyClientEvidence {
    pub interface: InterfaceId,
    pub ssid: Ssid,

    pub connected_bssid: Bssid,
    pub candidate_bssid: Bssid,

    /// RSSI текущей AP в последнем наблюдении.
    pub current_rssi_dbm: i16,

    /// RSSI соседней AP в последнем наблюдении.
    pub candidate_rssi_dbm: i16,

    /// Преимущество соседней AP в последнем наблюдении.
    pub signal_advantage_db: i32,

    /// Минимальное преимущество за весь период наблюдений.
    pub minimum_signal_advantage_db: i32,

    pub consecutive_observations: usize,
    pub duration_ns: u64,
}

/// Результат оценки Sticky Client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StickyClientAssessment {
    InsufficientData(StickyClientUnavailableReason),
    NoEvidence,
    Suspected(StickyClientEvidence),
}

#[derive(Debug, Clone)]
struct NeighborSignal {
    bssid: Bssid,
    rssi_dbm: i16,
}

#[derive(Debug, Clone)]
struct Sample {
    interface: InterfaceId,
    ssid: Ssid,
    connected_bssid: Bssid,
    current_rssi_dbm: i16,
    neighbors: Vec<NeighborSignal>,
    monotonic_time_ns: u64,
    neighbor_rssi_unavailable: bool,
}

/// Получает значение измерения и проверяет его происхождение и актуальность.
fn measurement_value<'a, T>(
    measurement: &'a Measurement<T>,
    reference_source: &SourceId,
    reference_session: &MeasurementSessionId,
    reference_time_ns: u64,
    config: &StickyClientConfig,
) -> Result<&'a T, StickyClientUnavailableReason> {
    if &measurement.source != reference_source {
        return Err(StickyClientUnavailableReason::DifferentSources);
    }

    if &measurement.observed_at.session_id != reference_session {
        return Err(StickyClientUnavailableReason::DifferentSessions);
    }

    let measured_at = measurement.observed_at.monotonic_time_ns;

    if measured_at <= reference_time_ns {
        let age = reference_time_ns - measured_at;

        if age > config.max_measurement_age_ns {
            return Err(StickyClientUnavailableReason::StaleMeasurement);
        }
    } else {
        let future_offset = measured_at - reference_time_ns;

        if future_offset > config.max_measurement_age_ns {
            return Err(StickyClientUnavailableReason::FutureMeasurement);
        }
    }

    match &measurement.state {
        MeasurementState::Available(value) => Ok(value),
        MeasurementState::Unavailable(reason) => Err(
            StickyClientUnavailableReason::MeasurementUnavailable(*reason),
        ),
    }
}

/// Преобразует снимок подключения в пригодное для анализа наблюдение.
fn extract_sample(
    observation: &ConnectionObservation,
    config: &StickyClientConfig,
) -> Result<Sample, StickyClientUnavailableReason> {
    let reference_source = &observation.state.source;
    let reference_session = &observation.state.observed_at.session_id;
    let reference_time_ns = observation.state.observed_at.monotonic_time_ns;

    let connected = measurement_value(
        &observation.state,
        reference_source,
        reference_session,
        reference_time_ns,
        config,
    )?;

    let connected = match connected {
        ConnectionState::Connected(access_point) => access_point,
        ConnectionState::Disconnected => {
            return Err(StickyClientUnavailableReason::NotConnected);
        }
    };

    let current_rssi = connected
        .rssi
        .as_ref()
        .ok_or(StickyClientUnavailableReason::CurrentRssiUnavailable)?;

    let current_rssi = measurement_value(
        current_rssi,
        reference_source,
        reference_session,
        reference_time_ns,
        config,
    )?;

    let neighbors = measurement_value(
        &observation.neighbors,
        reference_source,
        reference_session,
        reference_time_ns,
        config,
    )?;

    let mut neighbor_signals = Vec::new();
    let mut neighbor_rssi_unavailable = false;

    for neighbor in neighbors {
        if neighbor.ssid != connected.ssid || neighbor.bssid == connected.bssid {
            continue;
        }

        let Some(rssi_measurement) = neighbor.rssi.as_ref() else {
            neighbor_rssi_unavailable = true;
            continue;
        };

        let rssi = match measurement_value(
            rssi_measurement,
            reference_source,
            reference_session,
            reference_time_ns,
            config,
        ) {
            Ok(rssi) => rssi,
            Err(_) => {
                neighbor_rssi_unavailable = true;
                continue;
            }
        };

        neighbor_signals.push(NeighborSignal {
            bssid: neighbor.bssid.clone(),
            rssi_dbm: rssi.as_dbm(),
        });
    }

    Ok(Sample {
        interface: observation.interface.clone(),
        ssid: connected.ssid.clone(),
        connected_bssid: connected.bssid.clone(),
        current_rssi_dbm: current_rssi.as_dbm(),
        neighbors: neighbor_signals,
        monotonic_time_ns: reference_time_ns,
        neighbor_rssi_unavailable,
    })
}

/// Оценивает наличие признаков Sticky Client по последовательности снимков.
///
/// Для положительного результата необходимы:
/// - одна и та же подключённая AP;
/// - одна и та же соседняя AP;
/// - одинаковый SSID;
/// - преимущество соседней AP не меньше заданного порога;
/// - необходимое количество последовательных наблюдений;
/// - необходимая продолжительность наблюдения.
pub fn assess_sticky_client(
    observations: &[ConnectionObservation],
    config: &StickyClientConfig,
) -> StickyClientAssessment {
    if config.min_signal_advantage_db <= 0
        || config.min_consecutive_observations < 2
        || config.min_duration_ns == 0
        || config.max_measurement_age_ns == 0
        || config.max_observation_gap_ns == 0
    {
        return StickyClientAssessment::InsufficientData(
            StickyClientUnavailableReason::InvalidConfiguration,
        );
    }

    if observations.len() < config.min_consecutive_observations {
        return StickyClientAssessment::InsufficientData(
            StickyClientUnavailableReason::InsufficientObservations,
        );
    }

    let first = &observations[0];

    let mut samples: Vec<Sample> = Vec::with_capacity(observations.len());

    for (index, observation) in observations.iter().enumerate() {
        if observation.interface != first.interface {
            return StickyClientAssessment::InsufficientData(
                StickyClientUnavailableReason::DifferentInterfaces,
            );
        }

        if observation.state.source != first.state.source {
            return StickyClientAssessment::InsufficientData(
                StickyClientUnavailableReason::DifferentSources,
            );
        }

        if observation.state.observed_at.session_id != first.state.observed_at.session_id {
            return StickyClientAssessment::InsufficientData(
                StickyClientUnavailableReason::DifferentSessions,
            );
        }

        if index > 0 {
            let previous = &observations[index - 1];

            let current_time = observation.state.observed_at.monotonic_time_ns;
            let previous_time = previous.state.observed_at.monotonic_time_ns;

            if current_time <= previous_time {
                return StickyClientAssessment::InsufficientData(
                    StickyClientUnavailableReason::NonIncreasingTime,
                );
            }

            if current_time - previous_time > config.max_observation_gap_ns {
                return StickyClientAssessment::InsufficientData(
                    StickyClientUnavailableReason::ObservationGapExceeded,
                );
            }
        }

        let sample = match extract_sample(observation, config) {
            Ok(sample) => sample,
            Err(reason) => {
                return StickyClientAssessment::InsufficientData(reason);
            }
        };

        if let Some(previous) = samples.last() {
            if previous.ssid != sample.ssid {
                return StickyClientAssessment::InsufficientData(
                    StickyClientUnavailableReason::DifferentNetworks,
                );
            }

            if previous.connected_bssid != sample.connected_bssid {
                return StickyClientAssessment::InsufficientData(
                    StickyClientUnavailableReason::ConnectionChanged,
                );
            }
        }

        samples.push(sample);
    }

    let first_sample = &samples[0];
    let last_sample = samples.last().unwrap();

    let duration_ns = last_sample.monotonic_time_ns - first_sample.monotonic_time_ns;

    if duration_ns < config.min_duration_ns {
        return StickyClientAssessment::InsufficientData(
            StickyClientUnavailableReason::InsufficientDuration,
        );
    }

    // Проверяем каждую соседнюю AP, найденную в первом наблюдении.
    for candidate in &first_sample.neighbors {
        let mut minimum_advantage = i32::MAX;
        let mut stable = true;

        for sample in &samples {
            let Some(neighbor) = sample
                .neighbors
                .iter()
                .find(|neighbor| neighbor.bssid == candidate.bssid)
            else {
                stable = false;
                break;
            };

            let advantage = i32::from(neighbor.rssi_dbm) - i32::from(sample.current_rssi_dbm);

            if advantage < config.min_signal_advantage_db {
                stable = false;
                break;
            }

            minimum_advantage = minimum_advantage.min(advantage);
        }

        if !stable {
            continue;
        }

        let last_neighbor = last_sample
            .neighbors
            .iter()
            .find(|neighbor| neighbor.bssid == candidate.bssid)
            .unwrap();

        let signal_advantage =
            i32::from(last_neighbor.rssi_dbm) - i32::from(last_sample.current_rssi_dbm);

        return StickyClientAssessment::Suspected(StickyClientEvidence {
            interface: first_sample.interface.clone(),
            ssid: first_sample.ssid.clone(),
            connected_bssid: first_sample.connected_bssid.clone(),
            candidate_bssid: candidate.bssid.clone(),
            current_rssi_dbm: last_sample.current_rssi_dbm,
            candidate_rssi_dbm: last_neighbor.rssi_dbm,
            signal_advantage_db: signal_advantage,
            minimum_signal_advantage_db: minimum_advantage,
            consecutive_observations: samples.len(),
            duration_ns,
        });
    }

    if samples
        .iter()
        .any(|sample| sample.neighbor_rssi_unavailable)
    {
        StickyClientAssessment::InsufficientData(
            StickyClientUnavailableReason::NeighborRssiUnavailable,
        )
    } else {
        StickyClientAssessment::NoEvidence
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use super::super::connection::{ConnectedAccessPoint, NeighborAccessPoint};
    use super::super::measurement::{
        Measurement, MeasurementSessionId, MeasurementState, ObservationTime,
    };
    use super::super::radio::RssiDbm;
    use super::super::radio_observation::FrequencyMhz;

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
            StickyClientAssessment::InsufficientData(
                StickyClientUnavailableReason::StaleMeasurement,
            ),
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
            StickyClientAssessment::InsufficientData(
                StickyClientUnavailableReason::FutureMeasurement,
            ),
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
            StickyClientAssessment::InsufficientData(
                StickyClientUnavailableReason::DifferentSources,
            ),
        );
    }

    #[test]
    fn rejects_different_observation_sources() {
        let mut observations = stable_observations();
        observations[1].state.source = SourceId("another.provider".to_string());

        assert_eq!(
            assess_sticky_client(&observations, &config()),
            StickyClientAssessment::InsufficientData(
                StickyClientUnavailableReason::DifferentSources,
            ),
        );
    }

    #[test]
    fn rejects_different_sessions() {
        let mut observations = stable_observations();

        observations[1].state.observed_at.session_id =
            MeasurementSessionId("another-session".to_string());

        assert_eq!(
            assess_sticky_client(&observations, &config()),
            StickyClientAssessment::InsufficientData(
                StickyClientUnavailableReason::DifferentSessions,
            ),
        );
    }

    #[test]
    fn rejects_non_increasing_observation_time() {
        let mut observations = stable_observations();

        observations[1].state.observed_at.monotonic_time_ns = 0;

        assert_eq!(
            assess_sticky_client(&observations, &config()),
            StickyClientAssessment::InsufficientData(
                StickyClientUnavailableReason::NonIncreasingTime,
            ),
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
            StickyClientAssessment::InsufficientData(
                StickyClientUnavailableReason::ConnectionChanged,
            ),
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
            StickyClientAssessment::InsufficientData(
                StickyClientUnavailableReason::DifferentNetworks,
            ),
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
}
