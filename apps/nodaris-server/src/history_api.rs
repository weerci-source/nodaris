use axum::{
    extract::Path,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get},
    Extension, Json, Router,
};
use nodaris_core::{
    error::{CoreError, ErrorCode},
    keys::history,
};

use super::{probe_api::local_request, radio_api::error_response};

pub(super) fn routes(port: u16) -> Router {
    Router::new()
        .route("/api/diagnostics/history", get(list))
        .route("/api/diagnostics/history/{id}", delete(remove))
        .layer(Extension(port))
}

async fn list(Extension(port): Extension<u16>, headers: HeaderMap) -> Response {
    if !local_request(&headers, port) {
        return forbidden();
    }
    reply(nodaris_linux::history::list().await)
}

async fn remove(
    Extension(port): Extension<u16>,
    headers: HeaderMap,
    id: Result<Path<String>, axum::extract::rejection::PathRejection>,
) -> Response {
    if !local_request(&headers, port) {
        return forbidden();
    }
    let Ok(Path(id)) = id else {
        return error_response(
            StatusCode::BAD_REQUEST,
            CoreError::validation(history::INVALID_ID),
        );
    };
    reply(nodaris_linux::history::delete(id).await)
}

fn forbidden() -> Response {
    error_response(
        StatusCode::FORBIDDEN,
        CoreError::permission_err(history::FORBIDDEN),
    )
}

fn reply<T: serde::Serialize>(result: Result<T, CoreError>) -> Response {
    match result {
        Ok(data) => ([(header::CACHE_CONTROL, "no-store")], Json(data)).into_response(),
        Err(error) => {
            let status = if error.code == ErrorCode::ValidationError {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            error_response(status, error)
        }
    }
}
