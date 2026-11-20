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
use serde::Deserialize;

use super::{probe_api::local_request, radio_api::error_response};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AirtimeRequest {
    interface: String,
}

pub(super) async fn observe(
    Extension(port): Extension<u16>,
    headers: HeaderMap,
    request: Result<Json<AirtimeRequest>, JsonRejection>,
) -> Response {
    if !local_request(&headers, port) {
        return error_response(
            StatusCode::FORBIDDEN,
            CoreError::permission_err(diagnostics::AIRTIME_FORBIDDEN),
        );
    }
    let Ok(Json(request)) = request else {
        return error_response(
            StatusCode::BAD_REQUEST,
            CoreError::validation(diagnostics::AIRTIME_INVALID_REQUEST),
        );
    };
    match nodaris_linux::observe_airtime(request.interface).await {
        Ok(series) => ([(header::CACHE_CONTROL, "no-store")], Json(series)).into_response(),
        Err(error) => {
            let status = match error.code {
                ErrorCode::ValidationError => StatusCode::BAD_REQUEST,
                ErrorCode::NetworkError => StatusCode::SERVICE_UNAVAILABLE,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            error_response(status, error)
        }
    }
}
