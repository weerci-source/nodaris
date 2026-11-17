use serde::{Deserialize, Serialize};

use super::super::connection::Ssid;
use super::super::measurement::UnavailableReason;
use super::super::radio_observation::{Bssid, InterfaceId};

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

/// Причина, по которой соседняя точка доступа
/// не может участвовать в оценке Sticky Client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeighborMeasurementIssue {
    /// BSSID соседней точки доступа.
    pub bssid: Bssid,

    /// Конкретная причина недоступности измерения.
    pub reason: StickyClientUnavailableReason,
}

/// Проблема измерения соседней AP с привязкой к наблюдению.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedNeighborIssue {
    /// Индекс наблюдения во входной последовательности (с нуля).
    pub observation_index: usize,

    /// Монотонное время наблюдения внутри сессии.
    pub monotonic_time_ns: u64,

    /// Проблема измерения.
    pub issue: NeighborMeasurementIssue,
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

/// Подробный результат диагностики Sticky Client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StickyClientReport {
    /// Итоговая диагностическая оценка.
    pub assessment: StickyClientAssessment,

    /// Количество наблюдений, переданных на анализ.
    pub observation_count: usize,

    /// Проблемы измерений соседних точек доступа.
    pub neighbor_issues: Vec<ObservedNeighborIssue>,
}
