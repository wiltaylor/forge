use axum::{routing::get, Json, Router};
use serde::Serialize;

#[derive(Serialize)]
struct Service {
    name: String,
    region: String,
    status: String,
}

async fn services() -> Json<Vec<Service>> {
    Json(vec![Service {
        name: "ingest".into(),
        region: "us-east-1".into(),
        status: "ok".into(),
    }])
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/api/services", get(services));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
