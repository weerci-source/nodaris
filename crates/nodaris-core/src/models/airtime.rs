use serde::{Deserialize, Serialize};

use super::measurement::{Measurement, MeasurementState, ObservationTime, UnavailableReason};
use super::radio_observation::{FrequencyMhz, InterfaceId};

/// Накопительные счётчики времени радиоканала.
///
/// Все значения нормализованы в микросекунды.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelSurveyCounters {
    /// Общее накопленное время наблюдения канала.
    pub total_time_us: u64,

    /// Накопленное время занятости канала.
    pub busy_time_us: u64,

    /// Накопленное время приёма, если доступно.
    pub rx_time_us: Option<u64>,

    /// Накопленное время передачи, если доступно.
    pub tx_time_us: Option<u64>,
}

/// Снимок счётчиков конкретного радиоканала.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChannelSurveySnapshot {
    pub interface: InterfaceId,
    pub frequency: FrequencyMhz,
    pub counters: Measurement<ChannelSurveyCounters>,
}

/// Занятость радиоканала в процентах.
///
/// Храним значение в сотых долях процента:
/// 4000 = 40.00%.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AirtimeUtilizationPercent(u16);

impl AirtimeUtilizationPercent {
    pub const MAX: u16 = 10_000;

    pub const fn basis_points(self) -> u16 {
        self.0
    }

    pub fn percent(self) -> f64 {
        f64::from(self.0) / 100.0
    }

    fn from_deltas(busy_delta_us: u64, total_delta_us: u64) -> Self {
        let numerator = u128::from(busy_delta_us) * 10_000;
        let denominator = u128::from(total_delta_us);

        // Округление до ближайшей сотой доли процента.
        let value = (numerator + denominator / 2) / denominator;

        Self(value.min(u128::from(Self::MAX)) as u16)
    }
}

/// Причина невозможности вычисления Airtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AirtimeUnavailableReason {
    CountersUnavailable(UnavailableReason),
    DifferentInterface,
    DifferentFrequency,
    DifferentSource,
    DifferentSessions,
    NonIncreasingTime,
    CounterReset,
    ZeroTotalDelta,
    BusyDeltaExceedsTotalDelta,
    InvalidSnapshotCounters,
}

/// Результат вычисления занятости канала.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AirtimeResult {
    pub utilization: AirtimeUtilizationPercent,
    pub busy_delta_us: u64,
    pub total_delta_us: u64,
    pub interval_ns: u64,
    pub started_at: ObservationTime,
    pub finished_at: ObservationTime,
}

/// Результат анализа двух снимков.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AirtimeCalculation {
    Available(AirtimeResult),
    Unavailable(AirtimeUnavailableReason),
}

/// Вычисляет занятость канала по двум последовательным снимкам.
pub fn calculate_airtime(
    previous: &ChannelSurveySnapshot,
    current: &ChannelSurveySnapshot,
) -> AirtimeCalculation {
    // Оба снимка должны относиться к одному интерфейсу.
    if previous.interface != current.interface {
        return AirtimeCalculation::Unavailable(AirtimeUnavailableReason::DifferentInterface);
    }

    // Сравниваем только один и тот же радиоканал.
    if previous.frequency != current.frequency {
        return AirtimeCalculation::Unavailable(AirtimeUnavailableReason::DifferentFrequency);
    }

    // Извлекаем доступные показания счётчиков.
    let previous_counters = match &previous.counters.state {
        MeasurementState::Available(value) => value,
        MeasurementState::Unavailable(reason) => {
            return AirtimeCalculation::Unavailable(AirtimeUnavailableReason::CountersUnavailable(
                *reason,
            ));
        }
    };

    let current_counters = match &current.counters.state {
        MeasurementState::Available(value) => value,
        MeasurementState::Unavailable(reason) => {
            return AirtimeCalculation::Unavailable(AirtimeUnavailableReason::CountersUnavailable(
                *reason,
            ));
        }
    };

    // Для расчёта нужны показания одного источника.
    if previous.counters.source != current.counters.source {
        return AirtimeCalculation::Unavailable(AirtimeUnavailableReason::DifferentSource);
    }

    let previous_time = &previous.counters.observed_at;
    let current_time = &current.counters.observed_at;

    if previous_time.session_id != current_time.session_id {
        return AirtimeCalculation::Unavailable(AirtimeUnavailableReason::DifferentSessions);
    }

    if current_time.monotonic_time_ns <= previous_time.monotonic_time_ns {
        return AirtimeCalculation::Unavailable(AirtimeUnavailableReason::NonIncreasingTime);
    }

    // В каждом снимке busy не может превышать total.
    if previous_counters.busy_time_us > previous_counters.total_time_us
        || current_counters.busy_time_us > current_counters.total_time_us
    {
        return AirtimeCalculation::Unavailable(AirtimeUnavailableReason::InvalidSnapshotCounters);
    }

    // Уменьшение накопительного счётчика считаем сбросом.
    // Автоматически вычислять переполнение пока не будем.
    if current_counters.total_time_us < previous_counters.total_time_us
        || current_counters.busy_time_us < previous_counters.busy_time_us
    {
        return AirtimeCalculation::Unavailable(AirtimeUnavailableReason::CounterReset);
    }

    let total_delta = current_counters.total_time_us - previous_counters.total_time_us;

    let busy_delta = current_counters.busy_time_us - previous_counters.busy_time_us;

    if total_delta == 0 {
        return AirtimeCalculation::Unavailable(AirtimeUnavailableReason::ZeroTotalDelta);
    }

    if busy_delta > total_delta {
        return AirtimeCalculation::Unavailable(
            AirtimeUnavailableReason::BusyDeltaExceedsTotalDelta,
        );
    }

    let interval_ns = current_time.monotonic_time_ns - previous_time.monotonic_time_ns;

    AirtimeCalculation::Available(AirtimeResult {
        utilization: AirtimeUtilizationPercent::from_deltas(busy_delta, total_delta),
        busy_delta_us: busy_delta,
        total_delta_us: total_delta,
        interval_ns,
        started_at: previous_time.clone(),
        finished_at: current_time.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::super::measurement::{MeasurementSessionId, ObservationTime, SourceId};
    use super::*;

    fn snapshot(total: u64, busy: u64, time: u64) -> ChannelSurveySnapshot {
        ChannelSurveySnapshot {
            interface: InterfaceId("wlan0".into()),
            frequency: FrequencyMhz(5180),
            counters: Measurement::available(
                ChannelSurveyCounters {
                    total_time_us: total,
                    busy_time_us: busy,
                    rx_time_us: None,
                    tx_time_us: None,
                },
                SourceId("linux.nl80211".into()),
                ObservationTime {
                    unix_time_ms: Some(1_800_000_000_000),
                    monotonic_time_ns: time,
                    session_id: MeasurementSessionId("test-session".into()),
                },
            ),
        }
    }

    #[test]
    fn calculates_airtime_utilization() {
        let previous = snapshot(100_000, 30_000, 1_000_000);
        let current = snapshot(110_000, 34_000, 2_000_000);

        let result = calculate_airtime(&previous, &current);

        match result {
            AirtimeCalculation::Available(result) => {
                assert_eq!(result.utilization.percent(), 40.0);
                assert_eq!(result.busy_delta_us, 4_000);
                assert_eq!(result.total_delta_us, 10_000);
                assert_eq!(result.started_at, previous.counters.observed_at);
                assert_eq!(result.finished_at, current.counters.observed_at);
            }
            other => panic!("Unexpected result: {other:?}"),
        }
    }

    #[test]
    fn rejects_counter_reset() {
        let previous = snapshot(100_000, 30_000, 1_000_000);
        let current = snapshot(500, 100, 2_000_000);

        assert_eq!(
            calculate_airtime(&previous, &current),
            AirtimeCalculation::Unavailable(AirtimeUnavailableReason::CounterReset,)
        );
    }

    #[test]
    fn rejects_different_frequencies() {
        let previous = snapshot(100_000, 30_000, 1_000_000);
        let mut current = snapshot(110_000, 34_000, 2_000_000);

        current.frequency = FrequencyMhz(2412);

        assert_eq!(
            calculate_airtime(&previous, &current),
            AirtimeCalculation::Unavailable(AirtimeUnavailableReason::DifferentFrequency,)
        );
    }

    #[test]
    fn rejects_zero_total_delta() {
        let previous = snapshot(100_000, 30_000, 1_000_000);
        let current = snapshot(100_000, 30_000, 2_000_000);

        assert_eq!(
            calculate_airtime(&previous, &current),
            AirtimeCalculation::Unavailable(AirtimeUnavailableReason::ZeroTotalDelta,)
        );
    }

    #[test]
    fn rejects_invalid_snapshot_counters() {
        let previous = snapshot(100_000, 30_000, 1_000_000);
        let current = snapshot(110_000, 120_000, 2_000_000);

        assert_eq!(
            calculate_airtime(&previous, &current),
            AirtimeCalculation::Unavailable(AirtimeUnavailableReason::InvalidSnapshotCounters,)
        );
    }
}
