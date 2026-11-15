use serde::{Deserialize, Serialize};

use super::measurement::{Measurement, MeasurementState, UnavailableReason};
use super::radio_observation::InterfaceId;

/// Семантика счётчика повторных передач.
///
/// Разные платформы могут считать разные события.
/// Нельзя объединять такие счётчики без нормализации.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetryCounterSemantics {
    /// Количество дополнительных попыток передачи.
    RetryAttempts,

    /// Количество кадров, для которых выполнялась
    /// хотя бы одна повторная передача.
    RetriedFrames,

    /// Количество кадров, переданных успешно
    /// после одной или нескольких повторных попыток.
    FramesRecoveredAfterRetry,
}

/// Сырые счётчики передачи.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryCounters {
    /// Общее количество попыток передачи.
    pub tx_attempts: Option<u64>,

    /// Количество повторных попыток.
    pub tx_retries: Option<u64>,

    /// Количество кадров, передача которых завершилась неудачей.
    pub tx_failed: Option<u64>,

    /// Семантика счётчика повторных передач.
    pub retry_semantics: RetryCounterSemantics,
}

/// Снимок счётчиков передачи интерфейса.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrySnapshot {
    pub interface: InterfaceId,
    pub counters: Measurement<RetryCounters>,
}

/// Причина невозможности расчёта показателя повторных передач.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryUnavailableReason {
    CountersUnavailable(UnavailableReason),
    DifferentInterface,
    DifferentSources,
    DifferentSessions,
    NonIncreasingTime,
    CounterReset,
    MissingRetryCounter,
    MissingAttemptCounter,
    MissingFailureCounter,
    InvalidCounterValues,
    IncompatibleSemantics,
}

/// Результат расчёта повторных передач.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryResult {
    /// Количество повторных попыток за интервал.
    pub retries_delta: u64,

    /// Количество попыток передачи за интервал.
    pub attempts_delta: u64,

    /// Количество неудачных передач за интервал.
    pub failed_delta: Option<u64>,

    /// Доля повторных попыток от общего числа попыток.
    ///
    /// None, если количество попыток равно нулю.
    pub retry_rate_percent: Option<u16>,

    /// Временной интервал между снимками.
    pub interval_ns: u64,

    /// Семантика счётчика.
    pub semantics: RetryCounterSemantics,
}

/// Результат сравнения двух снимков.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetryCalculation {
    Available(RetryResult),
    Unavailable(RetryUnavailableReason),
}

/// Вычисляет дельты счётчиков передачи.
pub fn calculate_retries(previous: &RetrySnapshot, current: &RetrySnapshot) -> RetryCalculation {
    if previous.interface != current.interface {
        return RetryCalculation::Unavailable(RetryUnavailableReason::DifferentInterface);
    }

    let previous_counters = match &previous.counters.state {
        MeasurementState::Available(value) => value,
        MeasurementState::Unavailable(reason) => {
            return RetryCalculation::Unavailable(RetryUnavailableReason::CountersUnavailable(
                *reason,
            ));
        }
    };

    let current_counters = match &current.counters.state {
        MeasurementState::Available(value) => value,
        MeasurementState::Unavailable(reason) => {
            return RetryCalculation::Unavailable(RetryUnavailableReason::CountersUnavailable(
                *reason,
            ));
        }
    };

    if previous.counters.source != current.counters.source {
        return RetryCalculation::Unavailable(RetryUnavailableReason::DifferentSources);
    }

    let previous_time = &previous.counters.observed_at;
    let current_time = &current.counters.observed_at;

    if previous_time.session_id != current_time.session_id {
        return RetryCalculation::Unavailable(RetryUnavailableReason::DifferentSessions);
    }

    if current_time.monotonic_time_ns <= previous_time.monotonic_time_ns {
        return RetryCalculation::Unavailable(RetryUnavailableReason::NonIncreasingTime);
    }

    if previous_counters.retry_semantics != current_counters.retry_semantics {
        return RetryCalculation::Unavailable(RetryUnavailableReason::IncompatibleSemantics);
    }

    let previous_retries = match previous_counters.tx_retries {
        Some(value) => value,
        None => {
            return RetryCalculation::Unavailable(RetryUnavailableReason::MissingRetryCounter);
        }
    };

    let current_retries = match current_counters.tx_retries {
        Some(value) => value,
        None => {
            return RetryCalculation::Unavailable(RetryUnavailableReason::MissingRetryCounter);
        }
    };

    let previous_attempts = match previous_counters.tx_attempts {
        Some(value) => value,
        None => {
            return RetryCalculation::Unavailable(RetryUnavailableReason::MissingAttemptCounter);
        }
    };

    let current_attempts = match current_counters.tx_attempts {
        Some(value) => value,
        None => {
            return RetryCalculation::Unavailable(RetryUnavailableReason::MissingAttemptCounter);
        }
    };

    let failed_delta = match (previous_counters.tx_failed, current_counters.tx_failed) {
        (Some(previous), Some(current)) => {
            if current < previous {
                return RetryCalculation::Unavailable(RetryUnavailableReason::CounterReset);
            }

            Some(current - previous)
        }
        _ => None,
    };

    if current_retries < previous_retries || current_attempts < previous_attempts {
        return RetryCalculation::Unavailable(RetryUnavailableReason::CounterReset);
    }

    let retries_delta = current_retries - previous_retries;
    let attempts_delta = current_attempts - previous_attempts;

    if retries_delta > attempts_delta {
        return RetryCalculation::Unavailable(RetryUnavailableReason::InvalidCounterValues);
    }

    let retry_rate_percent = if attempts_delta == 0 {
        None
    } else {
        Some(
            ((u128::from(retries_delta) * 100 + u128::from(attempts_delta) / 2)
                / u128::from(attempts_delta))
            .min(100) as u16,
        )
    };

    RetryCalculation::Available(RetryResult {
        retries_delta,
        attempts_delta,
        failed_delta,
        retry_rate_percent,
        interval_ns: current_time.monotonic_time_ns - previous_time.monotonic_time_ns,
        semantics: current_counters.retry_semantics,
    })
}

#[cfg(test)]
mod tests {
    use super::super::measurement::{MeasurementSessionId, ObservationTime, SourceId};
    use super::*;

    fn snapshot(
        attempts: Option<u64>,
        retries: Option<u64>,
        failed: Option<u64>,
        time: u64,
    ) -> RetrySnapshot {
        RetrySnapshot {
            interface: InterfaceId("wlan0".into()),
            counters: Measurement::available(
                RetryCounters {
                    tx_attempts: attempts,
                    tx_retries: retries,
                    tx_failed: failed,
                    retry_semantics: RetryCounterSemantics::RetryAttempts,
                },
                SourceId("linux.test".into()),
                ObservationTime {
                    unix_time_ms: Some(1_800_000_000_000),
                    monotonic_time_ns: time,
                    session_id: MeasurementSessionId("test-session".into()),
                },
            ),
        }
    }

    #[test]
    fn calculates_retry_rate() {
        let previous = snapshot(Some(1_000), Some(100), Some(5), 1_000_000);

        let current = snapshot(Some(2_000), Some(300), Some(15), 2_000_000);

        let result = calculate_retries(&previous, &current);

        match result {
            RetryCalculation::Available(result) => {
                assert_eq!(result.attempts_delta, 1_000);
                assert_eq!(result.retries_delta, 200);
                assert_eq!(result.failed_delta, Some(10));
                assert_eq!(result.retry_rate_percent, Some(20));
            }
            other => panic!("Unexpected result: {other:?}"),
        }
    }

    #[test]
    fn rejects_counter_reset() {
        let previous = snapshot(Some(1_000), Some(200), Some(10), 1_000_000);

        let current = snapshot(Some(100), Some(20), Some(1), 2_000_000);

        assert_eq!(
            calculate_retries(&previous, &current),
            RetryCalculation::Unavailable(RetryUnavailableReason::CounterReset,)
        );
    }

    #[test]
    fn rejects_different_interfaces() {
        let previous = snapshot(Some(1_000), Some(100), Some(5), 1_000_000);

        let mut current = snapshot(Some(2_000), Some(200), Some(10), 2_000_000);

        current.interface = InterfaceId("wlan1".into());

        assert_eq!(
            calculate_retries(&previous, &current),
            RetryCalculation::Unavailable(RetryUnavailableReason::DifferentInterface,)
        );
    }

    #[test]
    fn rejects_different_sources() {
        let previous = snapshot(Some(1_000), Some(100), Some(5), 1_000_000);

        let mut current = snapshot(Some(2_000), Some(200), Some(10), 2_000_000);

        current.counters.source = SourceId("another.provider".into());

        assert_eq!(
            calculate_retries(&previous, &current),
            RetryCalculation::Unavailable(RetryUnavailableReason::DifferentSources,)
        );
    }

    #[test]
    fn rejects_incompatible_semantics() {
        let previous = snapshot(Some(1_000), Some(100), Some(5), 1_000_000);

        let mut current = snapshot(Some(2_000), Some(200), Some(10), 2_000_000);

        if let MeasurementState::Available(counters) = &mut current.counters.state {
            counters.retry_semantics = RetryCounterSemantics::RetriedFrames;
        }

        assert_eq!(
            calculate_retries(&previous, &current),
            RetryCalculation::Unavailable(RetryUnavailableReason::IncompatibleSemantics,)
        );
    }

    #[test]
    fn rejects_missing_retry_counter() {
        let previous = snapshot(Some(1_000), None, Some(5), 1_000_000);

        let current = snapshot(Some(2_000), Some(200), Some(10), 2_000_000);

        assert_eq!(
            calculate_retries(&previous, &current),
            RetryCalculation::Unavailable(RetryUnavailableReason::MissingRetryCounter,)
        );
    }

    #[test]
    fn handles_zero_attempts() {
        let previous = snapshot(Some(1_000), Some(100), Some(5), 1_000_000);

        let current = snapshot(Some(1_000), Some(100), Some(5), 2_000_000);

        match calculate_retries(&previous, &current) {
            RetryCalculation::Available(result) => {
                assert_eq!(result.attempts_delta, 0);
                assert_eq!(result.retries_delta, 0);
                assert_eq!(result.retry_rate_percent, None);
            }
            other => panic!("Unexpected result: {other:?}"),
        }
    }
}
