use super::super::connection::ConnectionObservation;

use super::sample::{Sample, extract_sample};
use super::types::ObservedNeighborIssue;
use super::{
    StickyClientAssessment, StickyClientConfig, StickyClientEvidence, StickyClientReport,
    StickyClientUnavailableReason,
};

/// Оценивает наличие признаков Sticky Client по последовательности снимков.
///
/// Сохраняет прежний публичный API, возвращающий только итоговую оценку.
pub fn assess_sticky_client(
    observations: &[ConnectionObservation],
    config: &StickyClientConfig,
) -> StickyClientAssessment {
    assess_sticky_client_report(observations, config).assessment
}

/// Формирует подробный диагностический отчёт Sticky Client.
pub fn assess_sticky_client_report(
    observations: &[ConnectionObservation],
    config: &StickyClientConfig,
) -> StickyClientReport {
    let mut samples = Vec::with_capacity(observations.len());

    let assessment = assess_with_samples(observations, config, &mut samples);

    let neighbor_issues = samples
        .into_iter()
        .enumerate()
        .flat_map(|(observation_index, sample)| {
            sample
                .neighbor_issues
                .into_iter()
                .map(move |issue| ObservedNeighborIssue {
                    observation_index,
                    monotonic_time_ns: sample.monotonic_time_ns,
                    issue,
                })
        })
        .collect();
    StickyClientReport {
        assessment,
        observation_count: observations.len(),
        neighbor_issues,
    }
}

/// Внутренний алгоритм оценки Sticky Client.
///
/// Для положительного результата необходимы:
/// - одна и та же подключённая AP;
/// - одна и та же соседняя AP;
/// - одинаковый SSID;
/// - преимущество соседней AP не меньше заданного порога;
/// - необходимое количество последовательных наблюдений;
/// - необходимая продолжительность наблюдения.
///
/// Успешно принятые образцы сохраняются в `samples` для отчёта.
fn assess_with_samples(
    observations: &[ConnectionObservation],
    config: &StickyClientConfig,
    samples: &mut Vec<Sample>,
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

        for sample in samples.iter() {
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
        .any(|sample| !sample.neighbor_issues.is_empty())
    {
        StickyClientAssessment::InsufficientData(
            StickyClientUnavailableReason::NeighborRssiUnavailable,
        )
    } else {
        StickyClientAssessment::NoEvidence
    }
}
