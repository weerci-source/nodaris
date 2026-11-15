use serde::{Deserialize, Serialize};

/// Уникальный идентификатор источника измерения.
///
/// Примеры:
/// - linux.nl80211
/// - linux.network_manager
/// - windows.native_wifi
/// - android.wifi_manager
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceId(pub String);

/// Причина недоступности измерения.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnavailableReason {
    NotSupported,
    PermissionDenied,
    HardwareUnavailable,
    DriverUnavailable,
    NotConfigured,
    NotMeasured,
    InvalidMeasurement,
    TemporarilyUnavailable,
}

/// Состояние измерения.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", content = "value", rename_all = "snake_case")]
pub enum MeasurementState<T> {
    Available(T),
    Unavailable(UnavailableReason),
}

/// Идентификатор сессии измерений.
///
/// Монотонное время имеет смысл только в пределах
/// одной сессии измерений.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MeasurementSessionId(pub String);

/// Время получения измерения.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationTime {
    /// Unix timestamp в миллисекундах.
    /// Используется для отображения времени.
    pub unix_time_ms: Option<i64>,

    /// Монотонное время в наносекундах
    /// относительно начала сессии.
    /// Используется для вычисления временных интервалов.
    pub monotonic_time_ns: u64,

    /// Сессия, в которой получено измерение.
    pub session_id: MeasurementSessionId,
}

/// Универсальная оболочка измерения.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Measurement<T> {
    pub state: MeasurementState<T>,
    pub source: SourceId,
    pub observed_at: ObservationTime,
}

impl<T> Measurement<T> {
    /// Создаёт измерение с полученным значением.
    pub fn available(value: T, source: SourceId, observed_at: ObservationTime) -> Self {
        Self {
            state: MeasurementState::Available(value),
            source,
            observed_at,
        }
    }

    /// Создаёт измерение с указанием причины недоступности.
    pub fn unavailable(
        reason: UnavailableReason,
        source: SourceId,
        observed_at: ObservationTime,
    ) -> Self {
        Self {
            state: MeasurementState::Unavailable(reason),
            source,
            observed_at,
        }
    }

    /// Возвращает значение, если измерение доступно.
    pub fn value(&self) -> Option<&T> {
        match &self.state {
            MeasurementState::Available(value) => Some(value),
            MeasurementState::Unavailable(_) => None,
        }
    }

    /// Проверяет доступность измерения.
    pub fn is_available(&self) -> bool {
        matches!(self.state, MeasurementState::Available(_))
    }
}
