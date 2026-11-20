use std::{
    thread,
    time::{Duration, Instant},
};

use nodaris_core::{
    error::CoreError,
    input::valid_interface_name,
    keys::diagnostics,
    models::{
        airtime::{ChannelSurveySnapshot, calculate_airtime},
        diagnostic_report::ReportSection,
        measurement::UnavailableReason,
        radio_observation::InterfaceId,
    },
    radio_api::{AirtimeSample, AirtimeSection, AirtimeSeries, collection_reason, encode_airtime},
};

use super::{iw_link, survey};
use crate::{session::RunClock, try_radio_slot};

const DURATION: Duration = Duration::from_secs(30);
const GAP: Duration = Duration::from_secs(1);

pub async fn observe_airtime(interface: String) -> Result<AirtimeSeries, CoreError> {
    if !valid_interface_name(&interface) {
        return Err(CoreError::validation(diagnostics::INVALID_INTERFACE));
    }
    let permit = try_radio_slot()?;
    tokio::task::spawn_blocking(move || {
        // Keep the radio slot until all bounded commands finish even if the client disconnects.
        let _permit = permit;
        observe(InterfaceId(interface))
    })
    .await
    .map_err(|error| CoreError::internal(format!("Airtime worker: {error}")))
}

fn observe(interface: InterfaceId) -> AirtimeSeries {
    let clock = RunClock::new();
    let mut series = AirtimeSeries {
        interface: interface.0.clone(),
        frequency_mhz: None,
        source: survey::SOURCE,
        started_at: clock.now(),
        finished_at: clock.now(),
        samples: Vec::new(),
        stopped_reason: None,
    };
    let link = match iw_link::read_link_state(&interface.0) {
        Ok(iw_link::LinkState::Connected { link, .. }) => link,
        result => {
            series.stopped_reason = Some(match result {
                Err(reason) => collection_reason(reason),
                _ => "not_connected",
            });
            series.finished_at = clock.now();
            return series;
        }
    };
    series.frequency_mhz = Some(link.frequency_mhz);
    let read = || {
        let snapshot = survey::read_counters(&interface, &link, || clock.now());
        match iw_link::read_link(&interface.0) {
            Ok(current) if link.same_context(&current) => snapshot,
            Ok(_) | Err(UnavailableReason::NotMeasured) => Err(UnavailableReason::ContextChanged),
            Err(reason) => Err(reason),
        }
    };
    let started = Instant::now();
    let deadline = started + DURATION;
    series.started_at = clock.now();
    let mut previous = None;
    let baseline = read();
    if let Err(reason) = baseline
        && stops_observation(reason)
    {
        series.stopped_reason = Some(collection_reason(reason));
    } else {
        let _ = measure(&mut previous, baseline);
        let mut next = started + GAP;
        while next <= deadline {
            // Skip missed nominal slots without making back-to-back reads.
            if next <= Instant::now() {
                previous = None;
                next += GAP;
                continue;
            }
            thread::sleep(next.saturating_duration_since(Instant::now()));
            let reading = read();
            let stopped_reason = reading
                .as_ref()
                .err()
                .copied()
                .filter(|r| stops_observation(*r));
            let observed = reading.as_ref().map_or_else(
                |_| clock.now(),
                |sample| sample.counters.observed_at.clone(),
            );
            series.samples.push(AirtimeSample {
                elapsed_ns: observed
                    .monotonic_time_ns
                    .saturating_sub(series.started_at.monotonic_time_ns),
                reading: measure(&mut previous, reading),
            });
            if let Some(reason) = stopped_reason {
                series.stopped_reason = Some(collection_reason(reason));
                break;
            }
            next += GAP;
        }
    }
    series.finished_at = clock.now();
    series
}

fn stops_observation(reason: UnavailableReason) -> bool {
    matches!(
        reason,
        UnavailableReason::ContextChanged
            | UnavailableReason::HardwareUnavailable
            | UnavailableReason::PermissionDenied
            | UnavailableReason::NotSupported
    )
}

fn measure(
    previous: &mut Option<ChannelSurveySnapshot>,
    reading: Result<ChannelSurveySnapshot, UnavailableReason>,
) -> AirtimeSection {
    let result = match reading {
        Ok(current) => {
            let calculation = previous
                .as_ref()
                .map(|last| calculate_airtime(last, &current));
            *previous = Some(current);
            return match calculation {
                Some(result) => encode_airtime(ReportSection::Available(result)),
                None => AirtimeSection::Unavailable {
                    reason: "baseline_missing",
                },
            };
        }
        Err(reason) => {
            *previous = None;
            ReportSection::Unavailable(reason)
        }
    };
    encode_airtime(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nodaris_core::models::{
        airtime::ChannelSurveyCounters,
        measurement::{Measurement, MeasurementSessionId, ObservationTime, SourceId},
        radio_observation::FrequencyMhz,
    };

    fn snapshot(seconds: u64, active_ms: u64, busy_ms: u64) -> ChannelSurveySnapshot {
        ChannelSurveySnapshot {
            interface: InterfaceId("wlan0".into()),
            frequency: FrequencyMhz(5260),
            counters: Measurement::available(
                ChannelSurveyCounters {
                    total_time_us: active_ms * 1_000,
                    busy_time_us: busy_ms * 1_000,
                    rx_time_us: None,
                    tx_time_us: None,
                },
                SourceId(survey::SOURCE.into()),
                ObservationTime {
                    monotonic_time_ns: seconds * 1_000_000_000,
                    unix_time_ms: None,
                    session_id: MeasurementSessionId("airtime-test".into()),
                },
            ),
        }
    }

    #[test]
    fn missing_sample_breaks_the_sequence_and_zero_is_a_reading() {
        let mut previous = Some(snapshot(0, 1000, 100));
        assert!(matches!(
            measure(&mut previous, Ok(snapshot(1, 2000, 350))),
            AirtimeSection::Available {
                utilization_percent: 25.0,
                ..
            }
        ));
        assert!(matches!(
            measure(&mut previous, Err(UnavailableReason::NotMeasured)),
            AirtimeSection::Unavailable {
                reason: "not_measured"
            }
        ));
        assert!(matches!(
            measure(&mut previous, Ok(snapshot(3, 4000, 900))),
            AirtimeSection::Unavailable {
                reason: "baseline_missing"
            }
        ));
        assert!(matches!(
            measure(&mut previous, Ok(snapshot(4, 5000, 900))),
            AirtimeSection::Available {
                utilization_percent: 0.0,
                ..
            }
        ));
    }

    #[test]
    fn uses_fresh_baseline_after_a_counter_reset() {
        let mut previous = Some(snapshot(0, 4000, 1000));
        assert!(matches!(
            measure(&mut previous, Ok(snapshot(1, 1000, 0))),
            AirtimeSection::Unavailable {
                reason: "counter_reset"
            }
        ));
        assert!(matches!(
            measure(&mut previous, Ok(snapshot(2, 2000, 1000))),
            AirtimeSection::Available {
                utilization_percent: 100.0,
                ..
            }
        ));
    }
}
