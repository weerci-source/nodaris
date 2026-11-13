use serde::{Deserialize, Serialize};

/// Идентификатор источника измерения.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceId(pub String);

/// Причина отсутствия измерения.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnavailableReason {
    NotSupported,
    PermissionDenied,
    HardwareUnavailable,
    DriverUnavailable,
    NotConfigured,
    NotMeasured,
    InvalidMeasurement,
}

/// Состояние измерения.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", content = "value", rename_all = "snake_case")]
pub enum MeasurementState<T> {
    Available(T),
    Unavailable(UnavailableReason),
}

/// Единица измерения.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeasurementUnit {
    Dbm,
    Db,
    Percent,
    Milliseconds,
    Packets,
    Bytes,
    Microseconds,
    Count,
    None,
}

/// Измерение с метаданными.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Measurement<T> {
    pub state: MeasurementState<T>,
    pub unit: MeasurementUnit,
    /// Unix timestamp в миллисекундах.
    pub timestamp_ms: i64,
    pub source: SourceId,
}

impl<T> Measurement<T> {
    pub fn available(value: T, unit: MeasurementUnit, timestamp_ms: i64, source: SourceId) -> Self {
        Self {
            state: MeasurementState::Available(value),
            unit,
            timestamp_ms,
            source,
        }
    }

    pub fn unavailable(
        reason: UnavailableReason,
        unit: MeasurementUnit,
        timestamp_ms: i64,
        source: SourceId,
    ) -> Self {
        Self {
            state: MeasurementState::Unavailable(reason),
            unit,
            timestamp_ms,
            source,
        }
    }
}
