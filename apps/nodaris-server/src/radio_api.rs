use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use axum::{
    extract::{rejection::QueryRejection, Query},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use nodaris_core::{
    error::CoreError,
    keys,
    models::{
        diagnostic_report::{DiagnosticReport, ReportSection},
        measurement::{MeasurementSessionId, ObservationTime, UnavailableReason},
        radio_observation::{InterfaceId, SnrResult, SnrUnavailableReason},
        retries::{RetryCalculation, RetryCounterSemantics, RetryUnavailableReason},
    },
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::Semaphore;

static RADIO_SLOT: Semaphore = Semaphore::const_new(1);
static NEXT_SESSION: AtomicU64 = AtomicU64::new(0);
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(5);
// Five bounded commands plus the gap between TX samples.
const RADIO_RESPONSE_TIMEOUT: Duration = Duration::from_secs(14);
const MAX_SNR_TIME_DIFFERENCE_NS: u64 = 1_000_000;

#[derive(Deserialize)]
pub(super) struct RadioQuery {
    interface: String,
}

#[derive(Serialize)]
struct RadioResponse {
    interface: String,
    radio: RadioSection,
    retries: RetrySection,
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
        snr: SnrResponse,
    },
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum SnrResponse {
    Available { db: i32 },
    Unavailable { reason: &'static str },
}

pub(super) async fn radio_handler(query: Result<Query<RadioQuery>, QueryRejection>) -> Response {
    let interface = match query {
        Ok(Query(query)) if valid_interface_name(&query.interface) => query.interface,
        _ => {
            return error_response(
                StatusCode::BAD_REQUEST,
                CoreError::validation(keys::diagnostics::INVALID_INTERFACE),
            );
        }
    };

    let Ok(permit) = RADIO_SLOT.try_acquire() else {
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            CoreError::network_err(keys::diagnostics::RADIO_BUSY),
        );
    };

    let interface_id = InterfaceId(interface.clone());
    let worker = tokio::task::spawn_blocking(move || {
        // Если HTTP-запрос перестанет ждать, слот всё равно останется занят
        // до окончания системного вызова.
        let _permit = permit;
        let started_at = Instant::now();
        let session = NEXT_SESSION.fetch_add(1, Ordering::Relaxed);

        nodaris_linux::collect_radio_report(
            &interface_id,
            || ObservationTime {
                unix_time_ms: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .ok()
                    .and_then(|elapsed| i64::try_from(elapsed.as_millis()).ok()),
                monotonic_time_ns: u64::try_from(started_at.elapsed().as_nanos())
                    .unwrap_or(u64::MAX),
                session_id: MeasurementSessionId(format!("http-radio-{session}")),
            },
            MAX_SNR_TIME_DIFFERENCE_NS,
        )
    });

    match tokio::time::timeout(RADIO_RESPONSE_TIMEOUT, worker).await {
        Ok(Ok(report)) => (
            [(header::CACHE_CONTROL, "no-store")],
            Json(encode_radio(interface, report)),
        )
            .into_response(),
        Ok(Err(error)) => {
            eprintln!("Radio worker failed: {error}");
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                CoreError::internal("Radio worker failed"),
            )
        }
        Err(_) => error_response(
            StatusCode::GATEWAY_TIMEOUT,
            CoreError::network_err(keys::diagnostics::RADIO_TIMEOUT),
        ),
    }
}

pub(super) fn valid_interface_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 15
        && !name.starts_with('-')
        && !matches!(name, "." | "..")
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

fn encode_radio(interface: String, report: DiagnosticReport) -> RadioResponse {
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
    }
}

pub(super) fn error_response(status: StatusCode, error: CoreError) -> Response {
    (
        status,
        Json(json!({
            "ok": false,
            "error": {
                "key": error.key,
                "code": error.code,
                "severity": error.severity,
                "args": error.args,
            },
        })),
    )
        .into_response()
}

pub(super) async fn interfaces_handler() -> Response {
    let Ok(permit) = RADIO_SLOT.try_acquire() else {
        return error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            CoreError::network_err(keys::diagnostics::RADIO_BUSY),
        );
    };

    let worker = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        nodaris_linux::list_wifi_interfaces()
    });

    match tokio::time::timeout(RESPONSE_TIMEOUT, worker).await {
        Ok(Ok(Ok(interfaces))) => {
            let interfaces: Vec<_> = interfaces
                .into_iter()
                .map(|interface| {
                    json!({
                        "name": interface.name,
                        "connected": interface.connected,
                    })
                })
                .collect();

            (
                [(header::CACHE_CONTROL, "no-store")],
                Json(json!({ "interfaces": interfaces })),
            )
                .into_response()
        }
        Ok(Ok(Err(error))) => {
            let status = if matches!(
                &error.code,
                nodaris_core::error::ErrorCode::PermissionDenied
            ) {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::SERVICE_UNAVAILABLE
            };

            error_response(status, error)
        }
        Ok(Err(error)) => {
            eprintln!("Wi-Fi interface worker failed: {error}");
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                CoreError::internal("Wi-Fi interface worker failed"),
            )
        }
        Err(_) => error_response(
            StatusCode::GATEWAY_TIMEOUT,
            CoreError::network_err(keys::diagnostics::RADIO_TIMEOUT),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nodaris_core::models::radio_observation::{FrequencyMhz, RadioObservation};

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
                    "snr": {
                        "status": "unavailable",
                        "reason": "rssi_unavailable",
                    },
                },
                "retries": { "status": "not_collected" },
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
            }),
        );
    }

    #[test]
    fn json_keeps_zero_delta_and_omits_the_unmeasured_rate() {
        use nodaris_core::models::retries::RetryDeltas;

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
