use axum::{
    extract::rejection::JsonRejection,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Extension, Json,
};
use nodaris_core::{
    error::{CoreError, ErrorCode},
    keys::diagnostics,
};
use serde::{Deserialize, Serialize};

use super::{probe_api::local_request, radio_api::error_response};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EthernetRequest {
    interface: String,
    target: String,
}

pub(super) async fn interfaces_handler(
    Extension(port): Extension<u16>,
    headers: HeaderMap,
) -> Response {
    if !local_request(&headers, port) {
        return forbidden();
    }
    let result = tokio::task::spawn_blocking(nodaris_linux::list_ethernet_links)
        .await
        .map_err(|error| CoreError::internal(format!("Ethernet list worker: {error}")))
        .and_then(|result| result);
    reply(result)
}

pub(super) async fn ethernet_handler(
    Extension(port): Extension<u16>,
    headers: HeaderMap,
    request: Result<Json<EthernetRequest>, JsonRejection>,
) -> Response {
    if !local_request(&headers, port) {
        return forbidden();
    }
    let Ok(Json(request)) = request else {
        return reply::<()>(Err(CoreError::validation(
            diagnostics::ETHERNET_INVALID_REQUEST,
        )));
    };
    reply(nodaris_linux::history::run_ethernet(request.interface, request.target).await)
}

fn forbidden() -> Response {
    error_response(
        StatusCode::FORBIDDEN,
        CoreError::permission_err(diagnostics::ETHERNET_FORBIDDEN),
    )
}

fn reply<T: Serialize>(result: Result<T, CoreError>) -> Response {
    match result {
        Ok(data) => ([(header::CACHE_CONTROL, "no-store")], Json(data)).into_response(),
        Err(error) => {
            let status = match error.code {
                ErrorCode::ValidationError => StatusCode::BAD_REQUEST,
                ErrorCode::PermissionDenied => StatusCode::FORBIDDEN,
                ErrorCode::HardwareError | ErrorCode::NetworkError => {
                    StatusCode::SERVICE_UNAVAILABLE
                }
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            error_response(status, error)
        }
    }
}
