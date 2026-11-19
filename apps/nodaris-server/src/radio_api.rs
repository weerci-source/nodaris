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
    },
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::Semaphore;

static RADIO_SLOT: Semaphore = Semaphore::const_new(1);
static NEXT_SESSION: AtomicU64 = AtomicU64::new(0);
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_SNR_TIME_DIFFERENCE_NS: u64 = 1_000_000;

#[derive(Deserialize)]
pub(super) struct RadioQuery {
    interface: String,
}

#[derive(Serialize)]
struct RadioResponse {
    interface: String,
    radio: RadioSection,
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

    match tokio::time::timeout(RESPONSE_TIMEOUT, worker).await {
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

fn valid_interface_name(name: &str) -> bool {
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

    RadioResponse { interface, radio }
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

fn error_response(status: StatusCode, error: CoreError) -> Response {
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
                    "snr": {
                        "status": "unavailable",
                        "reason": "rssi_unavailable",
                    },
                },
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
            }),
        );
    }
}
