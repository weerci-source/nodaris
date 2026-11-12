use axum::{Json, Router, routing::get};
use serde::Serialize;
use tower_http::{cors::CorsLayer, services::ServeDir};

#[derive(Serialize)]
struct Status {
    name: String,
    version: String,
    status: String,
}

async fn get_status() -> Json<Status> {
    Json(Status {
        name: "Nodaris".to_string(),
        version: "0.1.0".to_string(),
        status: "Scanning".to_string(),
    })
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/api/status", get(get_status))
        // Раздаем статику из папки ui/dist
        .nest_service("/", ServeDir::new("../../ui/dist"))
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    println!("Nodaris server running on http://localhost:8080");
    axum::serve(listener, app).await.unwrap();
}
