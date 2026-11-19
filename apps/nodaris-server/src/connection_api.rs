use axum::{
    extract::rejection::JsonRejection,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json,
};
use nodaris_core::{error::CoreError, keys::diagnostics};
use serde::Deserialize;

use super::{
    probe_api::local_request,
    radio_api::{error_response, valid_interface_name},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ConnectionRequest {
    interface: String,
}

pub(super) async fn connection_handler(
    Extension(port): Extension<u16>,
    headers: HeaderMap,
    request: Result<Json<ConnectionRequest>, JsonRejection>,
) -> Response {
    if !local_request(&headers, port) {
        return error_response(
            StatusCode::FORBIDDEN,
            CoreError::permission_err(diagnostics::CONNECTION_FORBIDDEN),
        );
    }
    let interface = match request {
        Ok(Json(request)) if valid_interface_name(&request.interface) => request.interface,
        _ => {
            return error_response(
                StatusCode::BAD_REQUEST,
                CoreError::validation(diagnostics::INVALID_INTERFACE),
            )
        }
    };
    match nodaris_linux::monitor_connection(interface).await {
        Ok(report) => ([(header::CACHE_CONTROL, "no-store")], Json(report)).into_response(),
        Err(error) => {
            let status = if error.key == diagnostics::CONNECTION_BUSY {
                StatusCode::SERVICE_UNAVAILABLE
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            error_response(status, error)
        }
    }
}
