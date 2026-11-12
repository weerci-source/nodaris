use axum::{http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use serde_json::json;
use tower_http::{cors::CorsLayer, services::ServeDir};

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

    let app = Router::new()
        .route("/api/scan", get(scan_handler))
        // ИСПОЛЬЗУЕМ fallback_service вместо nest_service
        .fallback_service(ServeDir::new(static_dir))
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    println!("Nodaris server running on http://localhost:8080");
    axum::serve(listener, app).await.unwrap();
}
