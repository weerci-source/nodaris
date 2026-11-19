mod probe_api;
mod radio_api;

use std::net::Ipv4Addr;

use axum::{
    extract::DefaultBodyLimit,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Extension, Json, Router,
};
use serde_json::json;
use tower_http::services::ServeDir;

async fn scan_handler() -> impl IntoResponse {
    match nodaris_linux::scan_wifi() {
        Ok(aps) => (StatusCode::OK, Json(aps)).into_response(),
        Err(e) => {
            // Логируем техническую часть
            if let Some(tech) = e.technical_message {
                eprintln!("[Tech Error] {}", tech);
            }

            // Отдаем структурированную ошибку
            let err_body = json!({
                "ok": false,
                "error": {
                    "key": e.key,
                    "code": e.code,
                    "severity": e.severity,
                    "args": e.args
                }
            });
            (StatusCode::INTERNAL_SERVER_ERROR, Json(err_body)).into_response()
        }
    }
}

#[tokio::main]
async fn main() {
    // ВАЖНО: Путь к статике.
    // Если вы запускаете `cargo run -p nodaris-server` из корня проекта (nodaris/),
    // то Svelte-проект лежит в папке "ui/dist", а НЕ "../../ui/dist".
    // Путь "../../ui/dist" был бы правильным, если бы вы запускали бинарник
    // из папки apps/nodaris-server/.
    let static_dir = "ui/dist";
    let port = match std::env::var("NODARIS_PORT") {
        Ok(value) => value
            .parse::<u16>()
            .expect("NODARIS_PORT must be a valid port"),
        Err(std::env::VarError::NotPresent) => 8080,
        Err(error) => panic!("Invalid NODARIS_PORT: {error}"),
    };
    let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, port))
        .await
        .unwrap();
    let listen_addr = listener.local_addr().unwrap();

    let app = Router::new()
        .route("/api/scan", get(scan_handler))
        .route(
            "/api/diagnostics/probes",
            post(probe_api::probe_handler)
                .layer::<_, std::convert::Infallible>(DefaultBodyLimit::max(1024))
                .layer(Extension(listen_addr.port())),
        )
        .route("/api/diagnostics/radio", get(radio_api::radio_handler))
        .route(
            "/api/diagnostics/interfaces",
            get(radio_api::interfaces_handler),
        )
        .fallback_service(ServeDir::new(static_dir));

    println!("Nodaris server running on http://{listen_addr}");
    axum::serve(listener, app).await.unwrap();
}
