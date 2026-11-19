use serde::Serialize;

use crate::models::{
    airtime::{AirtimeCalculation, AirtimeUnavailableReason},
    diagnostic_report::{DiagnosticReport, ReportSection},
    measurement::{Measurement, MeasurementState, ObservationTime, UnavailableReason},
    radio::NoiseDbm,
    radio_observation::{SnrResult, SnrUnavailableReason},
    retries::{RetryCalculation, RetryCounterSemantics, RetryUnavailableReason},
};

#[derive(Serialize)]
pub struct RadioResponse {
    interface: String,
    radio: RadioSection,
    retries: RetrySection,
    airtime: AirtimeSection,
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum AirtimeSection {
    NotCollected,
    Unavailable {
        reason: &'static str,
    },
    Available {
        utilization_percent: f64,
        busy_delta_us: u64,
        total_delta_us: u64,
        interval_ns: u64,
        started_at: ObservationTime,
        finished_at: ObservationTime,
    },
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum RetrySection {
    NotCollected,
    Unavailable {
        reason: &'static str,
    },
    Available {
        retries_delta: Option<u64>,
        failed_delta: Option<u64>,
        attempts_delta: Option<u64>,
        retry_rate_percent: Option<u16>,
        interval_ns: u64,
        semantics: RetryCounterSemantics,
    },
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum RadioSection {
    NotCollected,
    Unavailable {
        reason: &'static str,
    },
    Available {
        frequency_mhz: Option<u32>,
        signal_quality_percent: Option<u8>,
        rssi_dbm: Option<i16>,
        rssi_source: Option<String>,
        noise: NoiseResponse,
        snr: SnrResponse,
    },
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum SnrResponse {
    Available { db: i32 },
    Unavailable { reason: &'static str },
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum NoiseResponse {
    Available { dbm: i16, source: String },
    Unavailable { reason: &'static str },
}

pub fn encode_radio(interface: String, report: DiagnosticReport) -> RadioResponse {
    let radio = match report.radio {
        ReportSection::NotCollected => RadioSection::NotCollected,
        ReportSection::Unavailable(reason) => RadioSection::Unavailable {
            reason: collection_reason(reason),
        },
        ReportSection::Available(radio) => RadioSection::Available {
            frequency_mhz: radio.observation.frequency.map(|value| value.0),
            signal_quality_percent: radio
                .observation
                .signal_quality
                .as_ref()
                .and_then(|measurement| measurement.value())
                .map(|value| value.value()),
            rssi_dbm: radio
                .observation
                .rssi
                .as_ref()
                .and_then(|measurement| measurement.value())
                .map(|rssi| rssi.as_dbm()),
            rssi_source: radio.observation.rssi.and_then(|measurement| {
                measurement
                    .value()
                    .is_some()
                    .then_some(measurement.source.0)
            }),
            noise: encode_noise(radio.observation.noise),
            snr: match radio.snr {
                SnrResult::Available(value) => SnrResponse::Available { db: value.value() },
                SnrResult::Unavailable(reason) => SnrResponse::Unavailable {
                    reason: match reason {
                        SnrUnavailableReason::RssiUnavailable => "rssi_unavailable",
                        SnrUnavailableReason::NoiseUnavailable => "noise_unavailable",
                        SnrUnavailableReason::DifferentSessions => "different_sessions",
                        SnrUnavailableReason::TimeMismatch => "time_mismatch",
                        SnrUnavailableReason::MissingFrequency => "missing_frequency",
                    },
                },
            },
        },
    };

    RadioResponse {
        interface,
        radio,
        retries: encode_retries(report.retries),
        airtime: encode_airtime(report.airtime),
    }
}

fn encode_noise(noise: Option<Measurement<NoiseDbm>>) -> NoiseResponse {
    match noise {
        Some(measurement) => match measurement.state {
            MeasurementState::Available(value) => NoiseResponse::Available {
                dbm: value.value(),
                source: measurement.source.0,
            },
            MeasurementState::Unavailable(reason) => NoiseResponse::Unavailable {
                reason: collection_reason(reason),
            },
        },
        None => NoiseResponse::Unavailable {
            reason: "not_measured",
        },
    }
}

fn encode_airtime(airtime: ReportSection<AirtimeCalculation>) -> AirtimeSection {
    use AirtimeUnavailableReason as Reason;
    match airtime {
        ReportSection::NotCollected => AirtimeSection::NotCollected,
        ReportSection::Unavailable(reason) => AirtimeSection::Unavailable {
            reason: collection_reason(reason),
        },
        ReportSection::Available(AirtimeCalculation::Unavailable(reason)) => {
            AirtimeSection::Unavailable {
                reason: match reason {
                    Reason::CountersUnavailable(reason) => collection_reason(reason),
                    Reason::CounterReset => "counter_reset",
                    Reason::ZeroTotalDelta => "zero_total_delta",
                    Reason::DifferentInterface | Reason::DifferentFrequency => "context_changed",
                    Reason::DifferentSessions
                    | Reason::DifferentSource
                    | Reason::NonIncreasingTime
                    | Reason::InvalidSnapshotCounters
                    | Reason::BusyDeltaExceedsTotalDelta => "invalid_measurement",
                },
            }
        }
        ReportSection::Available(AirtimeCalculation::Available(result)) => {
            AirtimeSection::Available {
                utilization_percent: result.utilization.percent(),
                busy_delta_us: result.busy_delta_us,
                total_delta_us: result.total_delta_us,
                interval_ns: result.interval_ns,
                started_at: result.started_at,
                finished_at: result.finished_at,
            }
        }
    }
}

fn encode_retries(retries: ReportSection<RetryCalculation>) -> RetrySection {
    use RetryUnavailableReason as Reason;

    match retries {
        ReportSection::NotCollected => RetrySection::NotCollected,
        ReportSection::Unavailable(reason) => RetrySection::Unavailable {
            reason: collection_reason(reason),
        },
        ReportSection::Available(RetryCalculation::Unavailable(reason)) => {
            RetrySection::Unavailable {
                reason: match reason {
                    Reason::ConnectionChanged => "connection_changed",
                    Reason::AssociationUnverified => "association_unverified",
                    Reason::CounterReset => "counter_reset",
                    Reason::MissingRetryCounter
                    | Reason::MissingFailureCounter
                    | Reason::MissingCounters => "missing_counters",
                    Reason::MissingAttemptCounter => "missing_attempt_counter",
                    Reason::DifferentInterface
                    | Reason::DifferentSources
                    | Reason::DifferentSessions
                    | Reason::NonIncreasingTime
                    | Reason::InvalidCounterValues
                    | Reason::IncompatibleSemantics => "invalid_measurement",
                    Reason::CountersUnavailable(reason) => collection_reason(reason),
                },
            }
        }
        ReportSection::Available(RetryCalculation::DeltasOnly(result)) => RetrySection::Available {
            retries_delta: result.retries_delta,
            failed_delta: result.failed_delta,
            attempts_delta: None,
            retry_rate_percent: None,
            interval_ns: result.interval_ns,
            semantics: result.semantics,
        },
        ReportSection::Available(RetryCalculation::Available(result)) => RetrySection::Available {
            retries_delta: Some(result.retries_delta),
            failed_delta: result.failed_delta,
            attempts_delta: Some(result.attempts_delta),
            retry_rate_percent: result.retry_rate_percent,
            interval_ns: result.interval_ns,
            semantics: result.semantics,
        },
    }
}

fn collection_reason(reason: UnavailableReason) -> &'static str {
    match reason {
        UnavailableReason::NotSupported => "not_supported",
        UnavailableReason::PermissionDenied => "permission_denied",
        UnavailableReason::HardwareUnavailable => "hardware_unavailable",
        UnavailableReason::DriverUnavailable => "driver_unavailable",
        UnavailableReason::NotConfigured => "not_configured",
        UnavailableReason::NotMeasured => "not_measured",
        UnavailableReason::InvalidMeasurement => "invalid_measurement",
        UnavailableReason::TemporarilyUnavailable => "temporarily_unavailable",
        UnavailableReason::ContextChanged => "context_changed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        measurement::MeasurementSessionId,
        radio_observation::{FrequencyMhz, InterfaceId, RadioObservation},
    };
    use serde_json::json;

    #[test]
    fn json_preserves_noise_and_the_airtime_window() {
        use crate::models::{
            airtime::{ChannelSurveyCounters, ChannelSurveySnapshot, calculate_airtime},
            measurement::SourceId,
            radio::RssiDbm,
        };

        let time = |offset| ObservationTime {
            unix_time_ms: None,
            monotonic_time_ns: offset,
            session_id: MeasurementSessionId("survey-test".into()),
        };
        let interface = InterfaceId("wlan0".into());
        let frequency = FrequencyMhz(5260);
        let survey_source = SourceId("linux.iw.survey".into());
        let sample = |total_time_us, busy_time_us, offset| ChannelSurveySnapshot {
            interface: interface.clone(),
            frequency,
            counters: Measurement::available(
                ChannelSurveyCounters {
                    total_time_us,
                    busy_time_us,
                    rx_time_us: None,
                    tx_time_us: None,
                },
                survey_source.clone(),
                time(offset),
            ),
        };
        let observation = RadioObservation {
            interface: interface.clone(),
            bssid: None,
            frequency: Some(frequency),
            rssi: Some(Measurement::available(
                RssiDbm::new(-67),
                SourceId("linux.iw".into()),
                time(1),
            )),
            noise: Some(Measurement::available(
                NoiseDbm::new(-95),
                survey_source.clone(),
                time(2),
            )),
            signal_quality: None,
        };
        let mut report =
            DiagnosticReport::from_radio(ReportSection::Available(observation), 250_000_000);
        report.airtime = ReportSection::Available(calculate_airtime(
            &sample(10_000_000, 3_000_000, 2),
            &sample(11_000_000, 3_250_000, 1_000_000_002),
        ));

        let json = serde_json::to_value(encode_radio(interface.0, report)).unwrap();
        assert_eq!(
            json["radio"]["noise"],
            json!({"status": "available", "dbm": -95, "source": "linux.iw.survey"})
        );
        assert_eq!(
            json["radio"]["snr"],
            json!({"status": "available", "db": 28})
        );
        let airtime = &json["airtime"];
        assert_eq!(airtime["status"], "available");
        assert_eq!(airtime["utilization_percent"], 25.0);
        assert_eq!(airtime["busy_delta_us"], 250_000);
        assert_eq!(airtime["total_delta_us"], 1_000_000);
        assert_eq!(airtime["started_at"]["monotonic_time_ns"], 2);
        assert_eq!(airtime["finished_at"]["monotonic_time_ns"], 1_000_000_002);
    }

    #[test]
    fn json_keeps_radio_available_when_snr_is_missing() {
        let observation = RadioObservation {
            interface: InterfaceId("wlan0".into()),
            bssid: None,
            frequency: Some(FrequencyMhz(5260)),
            rssi: None,
            noise: None,
            signal_quality: None,
        };
        let report = DiagnosticReport::from_radio(ReportSection::Available(observation), 0);

        assert_eq!(
            serde_json::to_value(encode_radio("wlan0".into(), report)).unwrap(),
            json!({
                "interface": "wlan0",
                "radio": {
                    "status": "available",
                    "frequency_mhz": 5260,
                    "signal_quality_percent": null,
                    "rssi_dbm": null,
                    "rssi_source": null,
                    "noise": { "status": "unavailable", "reason": "not_measured" },
                    "snr": {
                        "status": "unavailable",
                        "reason": "rssi_unavailable",
                    },
                },
                "retries": { "status": "not_collected" },
                "airtime": { "status": "not_collected" },
            }),
        );
    }

    #[test]
    fn json_distinguishes_collection_failure_from_missing_snr() {
        let report = DiagnosticReport::from_radio(
            ReportSection::Unavailable(UnavailableReason::NotMeasured),
            0,
        );

        assert_eq!(
            serde_json::to_value(encode_radio("wlan0".into(), report)).unwrap(),
            json!({
                "interface": "wlan0",
                "radio": {
                    "status": "unavailable",
                    "reason": "not_measured",
                },
                "retries": { "status": "not_collected" },
                "airtime": { "status": "not_collected" },
            }),
        );
    }

    #[test]
    fn json_keeps_zero_delta_and_omits_the_unmeasured_rate() {
        use crate::models::retries::RetryDeltas;

        let retries = encode_retries(ReportSection::Available(RetryCalculation::DeltasOnly(
            RetryDeltas {
                retries_delta: Some(0),
                failed_delta: None,
                interval_ns: 1_000_000_000,
                semantics: RetryCounterSemantics::RetryAttempts,
            },
        )));
        assert_eq!(
            serde_json::to_value(retries).unwrap(),
            json!({
                "status": "available",
                "retries_delta": 0,
                "failed_delta": null,
                "attempts_delta": null,
                "retry_rate_percent": null,
                "interval_ns": 1_000_000_000u64,
                "semantics": "retry_attempts",
            })
        );
    }
}
