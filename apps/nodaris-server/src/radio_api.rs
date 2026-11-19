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
pub(super) use nodaris_core::input::valid_interface_name;
use nodaris_core::{
    error::CoreError,
    keys,
    models::{
        measurement::{MeasurementSessionId, ObservationTime},
        radio_observation::InterfaceId,
    },
    radio_api::encode_radio,
};
use serde::Deserialize;
use serde_json::json;

static NEXT_SESSION: AtomicU64 = AtomicU64::new(0);
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(5);
// Eight bounded commands and two sampling gaps take at most 20 seconds.
const RADIO_RESPONSE_TIMEOUT: Duration = Duration::from_secs(22);
const MAX_SNR_TIME_DIFFERENCE_NS: u64 = 250_000_000;

#[derive(Deserialize)]
pub(super) struct RadioQuery {
    interface: String,
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

    let Ok(permit) = nodaris_linux::try_radio_slot() else {
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
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                CoreError::internal(format!("Radio worker: {error}")),
            )
        }
        Err(_) => error_response(
            StatusCode::GATEWAY_TIMEOUT,
            CoreError::network_err(keys::diagnostics::RADIO_TIMEOUT),
        ),
    }
}

pub(super) fn error_response(status: StatusCode, error: CoreError) -> Response {
    (
        status,
        Json(json!({
            "ok": false,
            "error": error.into_api(),
        })),
    )
        .into_response()
}

pub(super) async fn interfaces_handler() -> Response {
    let Ok(permit) = nodaris_linux::try_radio_slot() else {
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
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                CoreError::internal(format!("Wi-Fi interface worker: {error}")),
            )
        }
        Err(_) => error_response(
            StatusCode::GATEWAY_TIMEOUT,
            CoreError::network_err(keys::diagnostics::RADIO_TIMEOUT),
        ),
    }
}
