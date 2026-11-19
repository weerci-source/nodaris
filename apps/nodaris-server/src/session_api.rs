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
pub(super) struct SessionRequest {
    interface: String,
    target: Option<String>,
}

pub(super) async fn session_handler(
    Extension(port): Extension<u16>,
    headers: HeaderMap,
    request: Result<Json<SessionRequest>, JsonRejection>,
) -> Response {
    if !local_request(&headers, port) {
        return error_response(
            StatusCode::FORBIDDEN,
            CoreError::permission_err(diagnostics::SESSION_FORBIDDEN),
        );
    }
    let Ok(Json(request)) = request else {
        return error_response(
            StatusCode::BAD_REQUEST,
            CoreError::validation(diagnostics::SESSION_INVALID_REQUEST),
        );
    };
    match nodaris_linux::collect_session(request.interface, request.target).await {
        Ok(report) => ([(header::CACHE_CONTROL, "no-store")], Json(report)).into_response(),
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
