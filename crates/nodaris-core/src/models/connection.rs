use serde::{Deserialize, Serialize};

use super::measurement::{
    Measurement, MeasurementState, ObservationTime, SourceId, UnavailableReason,
};
use super::radio::RssiDbm;
use super::radio_observation::{Bssid, FrequencyMhz, InterfaceId};

/// SSID беспроводной сети.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Ssid(pub String);

/// Подключённая точка доступа.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectedAccessPoint {
    pub ssid: Ssid,
    pub bssid: Bssid,
    pub frequency: Option<FrequencyMhz>,
    pub rssi: Option<Measurement<RssiDbm>>,
}

/// Обнаруженная соседняя точка доступа.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NeighborAccessPoint {
    pub ssid: Ssid,
    pub bssid: Bssid,
    pub frequency: FrequencyMhz,
    pub rssi: Option<Measurement<RssiDbm>>,
}

impl NeighborAccessPoint {
    pub fn belongs_to_network(&self, ssid: &Ssid) -> bool {
        self.ssid == *ssid
    }
}

/// Фактическое состояние подключения.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConnectionState {
    Connected(ConnectedAccessPoint),
    Disconnected,
}

/// Снимок состояния подключения и соседних точек.
///
/// Состояние подключения и результат сканирования соседей
/// являются независимыми измерениями.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConnectionObservation {
    pub interface: InterfaceId,
    pub state: Measurement<ConnectionState>,
    pub neighbors: Measurement<Vec<NeighborAccessPoint>>,
}

/// Метод обнаружения смены точки подключения.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoamingDetectionMethod {
    SnapshotComparison,
    OperatingSystemEvent,
}

/// Событие переключения между точками доступа.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoamingEvent {
    pub interface: InterfaceId,
    pub ssid: Ssid,
    pub from_bssid: Bssid,
    pub to_bssid: Bssid,
    pub observed_at: ObservationTime,
    pub source: SourceId,
    pub detection_method: RoamingDetectionMethod,
}

/// Причина невозможности определить изменение подключения.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoamingUnavailableReason {
    PreviousStateUnavailable(UnavailableReason),
    CurrentStateUnavailable(UnavailableReason),
    DifferentInterfaces,
    DifferentSources,
    DifferentSessions,
    NonIncreasingTime,
    DifferentNetworks,
}

/// Результат сравнения двух снимков подключения.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RoamingDetection {
    /// Состояние подключения не изменилось.
    NoChange,

    /// Обнаружено переключение между AP одной сети.
    Roamed(RoamingEvent),

    /// Клиент установил подключение.
    ConnectionEstablished,

    /// Клиент потерял подключение.
    ConnectionLost,
}

/// Сравнивает два последовательных снимка.
///
/// Смена BSSID при сохранении SSID считается Roaming.
/// Подключение, отключение и переход в другую сеть
/// не классифицируются как Roaming.
pub fn detect_roaming(
    previous: &ConnectionObservation,
    current: &ConnectionObservation,
) -> Result<RoamingDetection, RoamingUnavailableReason> {
    if previous.interface != current.interface {
        return Err(RoamingUnavailableReason::DifferentInterfaces);
    }

    if previous.state.source != current.state.source {
        return Err(RoamingUnavailableReason::DifferentSources);
    }

    if previous.state.observed_at.session_id != current.state.observed_at.session_id {
        return Err(RoamingUnavailableReason::DifferentSessions);
    }

    if current.state.observed_at.monotonic_time_ns <= previous.state.observed_at.monotonic_time_ns {
        return Err(RoamingUnavailableReason::NonIncreasingTime);
    }

    let previous_state = match &previous.state.state {
        MeasurementState::Available(state) => state,
        MeasurementState::Unavailable(reason) => {
            return Err(RoamingUnavailableReason::PreviousStateUnavailable(*reason));
        }
    };

    let current_state = match &current.state.state {
        MeasurementState::Available(state) => state,
        MeasurementState::Unavailable(reason) => {
            return Err(RoamingUnavailableReason::CurrentStateUnavailable(*reason));
        }
    };

    match (previous_state, current_state) {
        (ConnectionState::Disconnected, ConnectionState::Disconnected) => {
            Ok(RoamingDetection::NoChange)
        }

        (ConnectionState::Disconnected, ConnectionState::Connected(_)) => {
            Ok(RoamingDetection::ConnectionEstablished)
        }

        (ConnectionState::Connected(_), ConnectionState::Disconnected) => {
            Ok(RoamingDetection::ConnectionLost)
        }

        (ConnectionState::Connected(previous_ap), ConnectionState::Connected(current_ap)) => {
            if previous_ap.ssid != current_ap.ssid {
                return Err(RoamingUnavailableReason::DifferentNetworks);
            }

            if previous_ap.bssid == current_ap.bssid {
                return Ok(RoamingDetection::NoChange);
            }

            Ok(RoamingDetection::Roamed(RoamingEvent {
                interface: current.interface.clone(),
                ssid: current_ap.ssid.clone(),
                from_bssid: previous_ap.bssid.clone(),
                to_bssid: current_ap.bssid.clone(),
                observed_at: current.state.observed_at.clone(),
                source: current.state.source.clone(),
                detection_method: RoamingDetectionMethod::SnapshotComparison,
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(
        interface: &str,
        ssid: &str,
        bssid: &str,
        source: &str,
        session: &str,
        monotonic_time_ns: u64,
    ) -> ConnectionObservation {
        let source = SourceId(source.to_string());

        let observed_at = ObservationTime {
            unix_time_ms: Some(1_000),
            monotonic_time_ns,
            session_id: super::super::measurement::MeasurementSessionId(session.to_string()),
        };

        let state = Measurement {
            state: MeasurementState::Available(ConnectionState::Connected(ConnectedAccessPoint {
                ssid: Ssid(ssid.to_string()),
                bssid: Bssid(bssid.to_string()),
                frequency: Some(FrequencyMhz(5180)),
                rssi: None,
            })),
            source: source.clone(),
            observed_at: observed_at.clone(),
        };

        let neighbors = Measurement {
            state: MeasurementState::Available(Vec::new()),
            source,
            observed_at,
        };

        ConnectionObservation {
            interface: InterfaceId(interface.to_string()),
            state,
            neighbors,
        }
    }

    #[test]
    fn detects_roaming_between_access_points() {
        let previous = observation(
            "wlan0",
            "Home",
            "AA:BB:CC:00:00:01",
            "linux.test",
            "session-1",
            100,
        );

        let current = observation(
            "wlan0",
            "Home",
            "AA:BB:CC:00:00:02",
            "linux.test",
            "session-1",
            200,
        );

        let result = detect_roaming(&previous, &current).unwrap();

        match result {
            RoamingDetection::Roamed(event) => {
                assert_eq!(event.from_bssid, Bssid("AA:BB:CC:00:00:01".to_string()));
                assert_eq!(event.to_bssid, Bssid("AA:BB:CC:00:00:02".to_string()));
            }
            _ => panic!("Expected roaming event"),
        }
    }

    #[test]
    fn detects_no_change_when_bssid_is_same() {
        let previous = observation(
            "wlan0",
            "Home",
            "AA:BB:CC:00:00:01",
            "linux.test",
            "session-1",
            100,
        );

        let current = observation(
            "wlan0",
            "Home",
            "AA:BB:CC:00:00:01",
            "linux.test",
            "session-1",
            200,
        );

        assert_eq!(
            detect_roaming(&previous, &current).unwrap(),
            RoamingDetection::NoChange
        );
    }

    #[test]
    fn detects_connection_established() {
        let mut previous = observation(
            "wlan0",
            "Home",
            "AA:BB:CC:00:00:01",
            "linux.test",
            "session-1",
            100,
        );

        previous.state.state = MeasurementState::Available(ConnectionState::Disconnected);

        let current = observation(
            "wlan0",
            "Home",
            "AA:BB:CC:00:00:02",
            "linux.test",
            "session-1",
            200,
        );

        assert_eq!(
            detect_roaming(&previous, &current).unwrap(),
            RoamingDetection::ConnectionEstablished
        );
    }

    #[test]
    fn detects_connection_lost() {
        let previous = observation(
            "wlan0",
            "Home",
            "AA:BB:CC:00:00:01",
            "linux.test",
            "session-1",
            100,
        );

        let mut current = observation(
            "wlan0",
            "Home",
            "AA:BB:CC:00:00:01",
            "linux.test",
            "session-1",
            200,
        );

        current.state.state = MeasurementState::Available(ConnectionState::Disconnected);

        assert_eq!(
            detect_roaming(&previous, &current).unwrap(),
            RoamingDetection::ConnectionLost
        );
    }

    #[test]
    fn detects_no_change_when_disconnected() {
        let mut previous = observation(
            "wlan0",
            "Home",
            "AA:BB:CC:00:00:01",
            "linux.test",
            "session-1",
            100,
        );

        let mut current = observation(
            "wlan0",
            "Home",
            "AA:BB:CC:00:00:01",
            "linux.test",
            "session-1",
            200,
        );

        previous.state.state = MeasurementState::Available(ConnectionState::Disconnected);

        current.state.state = MeasurementState::Available(ConnectionState::Disconnected);

        assert_eq!(
            detect_roaming(&previous, &current).unwrap(),
            RoamingDetection::NoChange
        );
    }

    #[test]
    fn rejects_different_interfaces() {
        let previous = observation(
            "wlan0",
            "Home",
            "AA:BB:CC:00:00:01",
            "linux.test",
            "session-1",
            100,
        );

        let current = observation(
            "wlan1",
            "Home",
            "AA:BB:CC:00:00:02",
            "linux.test",
            "session-1",
            200,
        );

        assert_eq!(
            detect_roaming(&previous, &current),
            Err(RoamingUnavailableReason::DifferentInterfaces)
        );
    }

    #[test]
    fn rejects_different_sources() {
        let previous = observation(
            "wlan0",
            "Home",
            "AA:BB:CC:00:00:01",
            "provider-a",
            "session-1",
            100,
        );

        let current = observation(
            "wlan0",
            "Home",
            "AA:BB:CC:00:00:02",
            "provider-b",
            "session-1",
            200,
        );

        assert_eq!(
            detect_roaming(&previous, &current),
            Err(RoamingUnavailableReason::DifferentSources)
        );
    }

    #[test]
    fn rejects_different_sessions() {
        let previous = observation(
            "wlan0",
            "Home",
            "AA:BB:CC:00:00:01",
            "linux.test",
            "session-1",
            100,
        );

        let current = observation(
            "wlan0",
            "Home",
            "AA:BB:CC:00:00:02",
            "linux.test",
            "session-2",
            200,
        );

        assert_eq!(
            detect_roaming(&previous, &current),
            Err(RoamingUnavailableReason::DifferentSessions)
        );
    }

    #[test]
    fn rejects_non_increasing_time() {
        let previous = observation(
            "wlan0",
            "Home",
            "AA:BB:CC:00:00:01",
            "linux.test",
            "session-1",
            200,
        );

        let current = observation(
            "wlan0",
            "Home",
            "AA:BB:CC:00:00:02",
            "linux.test",
            "session-1",
            100,
        );

        assert_eq!(
            detect_roaming(&previous, &current),
            Err(RoamingUnavailableReason::NonIncreasingTime)
        );
    }

    #[test]
    fn rejects_unavailable_previous_state() {
        let mut previous = observation(
            "wlan0",
            "Home",
            "AA:BB:CC:00:00:01",
            "linux.test",
            "session-1",
            100,
        );

        previous.state.state = MeasurementState::Unavailable(UnavailableReason::PermissionDenied);

        let current = observation(
            "wlan0",
            "Home",
            "AA:BB:CC:00:00:02",
            "linux.test",
            "session-1",
            200,
        );

        assert_eq!(
            detect_roaming(&previous, &current),
            Err(RoamingUnavailableReason::PreviousStateUnavailable(
                UnavailableReason::PermissionDenied
            ))
        );
    }

    #[test]
    fn rejects_unavailable_current_state() {
        let previous = observation(
            "wlan0",
            "Home",
            "AA:BB:CC:00:00:01",
            "linux.test",
            "session-1",
            100,
        );

        let mut current = observation(
            "wlan0",
            "Home",
            "AA:BB:CC:00:00:02",
            "linux.test",
            "session-1",
            200,
        );

        current.state.state =
            MeasurementState::Unavailable(UnavailableReason::TemporarilyUnavailable);

        assert_eq!(
            detect_roaming(&previous, &current),
            Err(RoamingUnavailableReason::CurrentStateUnavailable(
                UnavailableReason::TemporarilyUnavailable
            ))
        );
    }

    #[test]
    fn rejects_different_networks() {
        let previous = observation(
            "wlan0",
            "Home",
            "AA:BB:CC:00:00:01",
            "linux.test",
            "session-1",
            100,
        );

        let current = observation(
            "wlan0",
            "Guest",
            "AA:BB:CC:00:00:02",
            "linux.test",
            "session-1",
            200,
        );

        assert_eq!(
            detect_roaming(&previous, &current),
            Err(RoamingUnavailableReason::DifferentNetworks)
        );
    }
}
